mod device;
mod reg;

use crate::arch::mm::virt::map_to_flags;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::mm::virt::PageFlags;
use alloc::sync::Arc;
use spin::Once;

use self::reg::*;
use crate::arch::mm::phys::allocate_order;
use bit_field::BitField;

struct Ahci {}

impl Ahci {
    pub fn setup_port(&self, port: &mut HbaPort) {
        port.stop_cmd();

        let addr = allocate_order(1).unwrap().address();

        map_to_flags(
            addr.to_virt(),
            addr,
            PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
        );
        map_to_flags(
            addr.to_virt() + 0x1000,
            addr + 0x1000,
            PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
        );

        for i in 0..32 {
            let cmd_hdr = port.cmd_header_at(i);

            cmd_hdr.set_prdtl(8);
            cmd_hdr.set_prd_byte_count(0);
            cmd_hdr.set_cmd_tbl_base_addr(addr + 256 * i);
        }

        port.start_cmd();
    }

    pub fn test_read(&self, port: &mut HbaPort) {
        println!("Start test read");
        port.set_is(HbaPortISReg::all());

        let mut slots = port.sact() | port.ci();

        for i in 0..32 {
            if slots & 1 == 0 {
                let hdr = port.cmd_header_at(i);

                let mut flags = hdr.flags();
                flags.remove(HbaCmdHeaderFlags::W);
                flags.set_command_fis_length((core::mem::size_of::<FisRegH2D>() / 4) as u8);

                hdr.set_flags(flags);

                hdr.set_prdtl(1);

                let dest_buf = allocate_order(0).unwrap().address();

                let tbl = hdr.cmd_tbl();

                let prdt = tbl.prdt_entry_mut(0);

                prdt.set_data_byte_count(512 - 1);
                prdt.set_interrupt_on_completion(true);
                prdt.set_database_address(dest_buf);

                let fis = tbl.cfis_as_h2d_mut();

                fis.set_fis_type(FisType::RegH2D);
                fis.set_c(true);

                println!("HERE");
                fis.set_command(AtaCommand::AtaCommandReadDmaExt);
                fis.set_lba0(0);
                fis.set_lba1(0);
                fis.set_lba2(0);
                fis.set_device(1 << 6);
                fis.set_lba3(0);
                fis.set_lba4(0);
                fis.set_lba5(0);

                fis.set_count(1);

                //todo: wait here

                port.set_ci(1 << i); // issue cmd

                loop {
                    if port.ci() & (1 << i) == 0 {
                        println!("Wait..");
                        break;
                    }
                }

                println!("Read complete, mbr magic: 0x{:x}", unsafe {
                    (dest_buf.to_mapped() + 510).read_volatile::<u32>()
                });

                break;
            }

            slots >>= 1;
        }
    }
}

impl PciDeviceHandle for Ahci {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2922) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        println!("[ AHCI ] Ahci driver");
        pci_data.hdr().enable_bus_mastering();

        if let PciHeader::Type0(dhdr) = pci_data {
            println!("[ AHCI ] Base: {:0x}", dhdr.base_address5());
            println!("bar 0 0x{:x}", dhdr.base_address0());
            println!("bar 1 0x{:x}", dhdr.base_address1());
            println!("bar 2 0x{:x}", dhdr.base_address2());
            println!("bar 3 0x{:x}", dhdr.base_address3());
            println!("bar 4 0x{:x}", dhdr.base_address4());

            let mut mapped = PhysAddr(dhdr.base_address5() as usize).to_virt();

            map_to_flags(
                mapped,
                PhysAddr(dhdr.base_address5() as usize),
                PageFlags::NO_CACHE | PageFlags::WRT_THROUGH | PageFlags::WRITABLE,
            );

            //use crate::kernel::mm::virt::PageFlags;

            //crate::kernel::mm::map_flags(
            //    mapped,
            //    PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
            //);

            let hba = unsafe { mapped.read_mut::<HbaMem>() };

            //hba.set_ghc(hba.ghc() | HbaMemGhcReg::IE);

            println!("[ AHCI ] Ports: 0b{:b}", hba.pi());
            println!("[ AHCI ] Cap:   0b{:b}", hba.cap());
            println!("[ AHCI ] ghc  : 0x{:x}", hba.ghc());
            println!("[ AHCI ] vers : 0x{:x}", hba.vs());
            println!("[ AHCI ] cap2 : 0x{:x}", hba.cap2());

            let pi = hba.pi();

            for i in 0..32 {
                if pi.get_bit(i) {
                    let port = hba.port_mut(i);

                    println!("[ AHCI ] Port {} fb: {}", i, port.fb());
                    println!("[ AHCI ] Port {} cb: 0b{:b}", i, port.clb().0);

                    let sts = port.ssts();
                    let ipm = sts.interface_power_management();

                    let dev = sts.device_detection();

                    if let HbaPortSstsRegDet::PresentAndE = dev {
                        println!("Dev present and enabled");
                    }

                    if let HbaPortSstsRegIpm::Active = ipm {
                        println!("Dev active");

                        self.setup_port(port);

                        self.test_read(port);
                    }
                }
            }
        }

        let data = pci_data.hdr();
        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ AHCI ] Interrupt line: {}", p);
        }

        true
    }
}

static DEVICE: Once<Arc<Ahci>> = Once::new();

fn device() -> &'static Arc<Ahci> {
    DEVICE.get().unwrap()
}

fn init() {
    DEVICE.call_once(|| Arc::new(Ahci {}));

    register_pci_device(device().clone());
}

module_init!(init);
