use alloc::sync::Arc;

use bit_field::BitField;

use crate::arch::idt::add_shared_irq_handler;
use crate::arch::int::{set_active_high, set_irq_dest};
use crate::arch::mm::phys::allocate_order;
use crate::arch::mm::virt::map_to_flags;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::block::ahci::reg::*;
use crate::drivers::block::ahci::request::{make_request, DmaBuf, ReadRequest};
use crate::drivers::pci::PciHeader;
use crate::kernel::device::block::{register_blkdev, BlockDev, BlockDevice};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sync::Spin;
use alloc::string::String;

struct Cmd {
    req: Arc<ReadRequest>,
}

struct PortData {
    cmds: [Option<Cmd>; 32],
    port: VirtAddr,
}

pub struct Port {
    data: Spin<PortData>,
}

impl PortData {
    fn hba_port(&mut self) -> &mut HbaPort {
        unsafe { self.port.read_mut::<HbaPort>() }
    }

    fn handle_interrupt(&mut self) {
        let port = self.hba_port();
        let ci = port.ci();

        port.set_is(port.is());

        for (i, cmd) in self.cmds.iter_mut().enumerate() {
            if cmd.is_some() && !ci.get_bit(i) {
                let fin = cmd
                    .as_mut()
                    .unwrap()
                    .req
                    .incomplete
                    .fetch_sub(1, core::sync::atomic::Ordering::SeqCst)
                    == 1;

                if fin {
                    cmd.as_ref().unwrap().req.wq.notify_one();
                }

                *cmd = None;
            }
        }
    }

    fn find_cmd_slot(&mut self) -> Option<usize> {
        for i in 0..32 {
            if self.cmds[i].is_none() {
                return Some(i);
            }
        }

        None
    }

    fn read(&mut self, request: Arc<ReadRequest>) {
        let mut rem = request.count;
        let mut off = 0;
        let mut buf_off = 0;

        while rem > 0 {
            let slot = {
                let slot = self.find_cmd_slot().expect("No free cmd slots!");
                //println!("Use slot: {}", slot);

                let port = self.hba_port();

                let cnt = core::cmp::min(rem, 128);

                port.read(request.sector + off, cnt, &request.buf_vec[buf_off..], slot);

                rem -= cnt;
                off += cnt;
                buf_off += 8;

                slot
            };

            self.cmds[slot] = Some(Cmd {
                req: request.clone(),
            });

            request
                .incomplete
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        }
    }
}

impl Port {
    fn handle_interrupt(&self) {
        self.data.lock_irq().handle_interrupt();
    }
}

impl BlockDev for Port {
    fn read(&self, sector: usize, count: usize, dest: &mut [u8]) -> Option<usize> {
        if dest.len() < count * 512 {
            return None;
        }

        let req = Arc::new(make_request(sector, count));

        // post request and wait for completion.....
        self.data.lock_irq().read(req.clone());

        req.wq
            .wait_for(|| req.incomplete.load(core::sync::atomic::Ordering::SeqCst) == 0);

        let mut off = 0;
        let mut rem = count * 512;

        for buf in req.buf_vec.iter() {
            let cnt = core::cmp::min(rem, 0x2000);

            dest[off..off + cnt].copy_from_slice(unsafe {
                core::slice::from_raw_parts(buf.buf.to_mapped().0 as *const u8, cnt)
            });

            rem -= cnt;
            off += cnt;
        }

        Some(count * 512)
    }

    fn write(&self, _sector: usize, _buf: &[u8]) -> Option<usize> {
        unimplemented!()
    }
}

pub struct AhciDevice {
    ports: [Option<Arc<Port>>; 32],
    hba: VirtAddr,
}

impl HbaPort {
    pub fn read(&mut self, sector: usize, count: usize, buf: &[DmaBuf], slot: usize) -> bool {
        let hdr = self.cmd_header_at(slot);

        let mut flags = hdr.flags();
        flags.remove(HbaCmdHeaderFlags::W);
        flags.set_command_fis_length((core::mem::size_of::<FisRegH2D>() / 4) as u8);
        hdr.set_flags(flags);

        let l = ((count - 1) >> 4) + 1;
        hdr.set_prdtl(l);

        let tbl = hdr.cmd_tbl();

        let mut cnt = count;
        for pri in 0..l - 1 {
            let prdt = tbl.prdt_entry_mut(pri);

            prdt.set_database_address(buf[pri].buf);
            prdt.set_data_byte_count(8192 - 1);
            prdt.set_interrupt_on_completion(true);

            cnt -= 16;
        }

        let prdt = tbl.prdt_entry_mut(l - 1);

        prdt.set_data_byte_count((cnt * 512) - 1);
        prdt.set_interrupt_on_completion(true);
        prdt.set_database_address(buf[l - 1].buf);

        let fis = tbl.cfis_as_h2d_mut();

        fis.set_fis_type(FisType::RegH2D);
        fis.set_c(true);
        fis.set_command(AtaCommand::AtaCommandReadDmaExt);
        fis.set_lba(sector);
        fis.set_device(1 << 6);

        fis.set_count(count as u16);

        //todo: wait here

        self.set_ci(self.ci() | (1 << slot)); // issue cmd

        true
    }

    /*
    pub fn write(&mut self) {
        if let Some((hdr, i)) = self.find_cmd_slot() {
            let mut flags = hdr.flags();
            flags.insert(HbaCmdHeaderFlags::W);
            flags.set_command_fis_length((core::mem::size_of::<FisRegH2D>() / 4) as u8);

            hdr.set_flags(flags);

            hdr.set_prdtl(1);

            let dest_buf = allocate_order(0).unwrap().address();

            unsafe {
                core::slice::from_raw_parts_mut(dest_buf.to_mapped().0 as *mut u8, 0x1000)
                    .fill(0xA5);
            }

            let tbl = hdr.cmd_tbl();

            let prdt = tbl.prdt_entry_mut(0);

            prdt.set_data_byte_count(512 - 1);
            prdt.set_interrupt_on_completion(true);
            prdt.set_database_address(dest_buf);

            let fis = tbl.cfis_as_h2d_mut();

            fis.set_fis_type(FisType::RegH2D);
            fis.set_c(true);
            fis.set_command(AtaCommand::AtaCommandWriteDmaExt);
            fis.set_lba(0);
            fis.set_device(1 << 6);

            fis.set_count(1);

            //todo: wait here

            self.set_ci(1 << i); // issue cmd
        } else {
            panic!("[ AHCI ] No free cmd slot found");
        }
    }*/

    pub fn start(&mut self) {
        self.stop_cmd();

        let addr = allocate_order(1).unwrap().address();

        for o in (0..0x2000).step_by(0x1000) {
            map_to_flags(
                addr.to_virt() + o,
                addr + o,
                PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
            );
        }

        for i in 0..32 {
            let cmd_hdr = self.cmd_header_at(i);

            cmd_hdr.set_prdtl(8);
            cmd_hdr.set_prd_byte_count(0);
            cmd_hdr.set_cmd_tbl_base_addr(addr + 256 * i);
        }

        self.set_ie(HbaPortIEReg::all());

        self.start_cmd();
    }

    fn probe(&mut self, num: usize) -> bool {
        let sts = self.ssts();

        let ipm = sts.interface_power_management();
        let dev = sts.device_detection();

        if let (HbaPortSstsRegDet::PresentAndE, HbaPortSstsRegIpm::Active) = (dev, ipm) {
            println!("[ AHCI ] Enabling Ahci port {}", num);

            self.start();

            //self.read(0, 0, VirtAddr(0));

            return true;
        }

        false
    }
}

impl AhciDevice {
    pub fn new() -> AhciDevice {
        AhciDevice {
            ports: [None; 32],
            hba: VirtAddr(0),
        }
    }

    fn hba(&mut self) -> &mut HbaMem {
        unsafe { self.hba.read_mut::<HbaMem>() }
    }

    fn enable_interrupts(&mut self, pci_data: &PciHeader) {
        pci_data.hdr().enable_bus_mastering();

        let data = pci_data.hdr();
        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ AHCI ] Using interrupt: {}", p);

            set_irq_dest(p as u8, p as u8 + 32);
            set_active_high(p as u8, true);
            add_shared_irq_handler(p as usize + 32, super::ahci_handler);
        }
    }

    fn start_hba(&mut self) -> bool {
        let mut hba = self.hba();

        hba.set_ghc(hba.ghc() | HbaMemGhcReg::IE);

        let pi = hba.pi();

        for i in 0..32 {
            if pi.get_bit(i) {
                let port = hba.port_mut(i);
                if port.probe(i) {
                    let addr = port as *const _ as usize;
                    drop(port);
                    drop(hba);

                    let port_dev = Arc::new(Port {
                        data: Spin::new(PortData {
                            cmds: [None; 32],
                            port: VirtAddr(addr),
                        }),
                    });

                    if let Err(d) =
                        register_blkdev(BlockDevice::new(String::from("disk"), port_dev.clone()))
                    {
                        panic!("Failed to register blkdev {:?}", d);
                    }

                    self.ports[i] = Some(port_dev);

                    hba = self.hba();
                }
            }
        }

        true
    }

    pub fn start(&mut self, pci_data: &PciHeader) -> bool {
        if let PciHeader::Type0(dhdr) = pci_data {
            self.hba = PhysAddr(dhdr.base_address5() as usize).to_virt();

            map_to_flags(
                self.hba,
                PhysAddr(dhdr.base_address5() as usize),
                PageFlags::NO_CACHE | PageFlags::WRT_THROUGH | PageFlags::WRITABLE,
            );

            self.enable_interrupts(pci_data);

            self.start_hba();

            //self.hba().port_mut(0).read(0, 0, PhysAddr(0));

            return true;
        }

        false
    }

    pub fn handle_interrupt(&mut self) -> bool {
        let hba = self.hba();

        if hba.is() != 0 {
            //println!("AHCI: 0b{:b}", hba.is());
            hba.set_is(hba.is());

            if let Some(p) = &self.ports[0] {
                p.handle_interrupt();
            }
        }

        return false;
    }
}
