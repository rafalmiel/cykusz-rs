use alloc::sync::Arc;

use bit_field::BitField;

use regs::*;

use crate::arch::idt::add_shared_irq_handler;
use crate::arch::int::{set_active_high, set_irq_dest};
use crate::drivers::block::ata::request::{DmaBuf, DmaRequest};
use crate::drivers::block::ata::AtaCommand;
use crate::drivers::block::ide::ata_handler;
use crate::kernel::mm::{allocate_order, PhysAddr};
use crate::kernel::sync::Spin;
use crate::kernel::timer::busy_sleep;
use crate::kernel::utils::wait_queue::WaitQueue;

mod regs;

struct IdeChannelData {
    base: DevBaseReg,
    ctrl: DevCtrlReg,
    bmide: BusMasterReg,
    interrupt_nr: usize,
    prdt_addr: PhysAddr,
    active_cmd: Option<Arc<DmaRequest>>,
}

struct PrdTable<'a> {
    data: &'a mut [PrdEntry],
}

impl<'a> PrdTable<'a> {
    pub fn new(addr: PhysAddr, entries: usize) -> PrdTable<'a> {
        let e = unsafe { addr.to_mapped().as_slice_mut::<PrdEntry>(entries) };

        PrdTable::<'a> { data: e }
    }

    pub fn entry_at(&mut self, idx: usize) -> &mut PrdEntry {
        &mut self.data[idx]
    }

    pub fn load_dma(&mut self, buf: &[DmaBuf], sectors: usize) -> usize {
        let mut rem = sectors;

        for (i, b) in buf.iter().enumerate() {
            let cur_sectors = b.sectors();

            let is_last = (i == buf.len() - 1) || (rem == cur_sectors);

            let entry = self.entry_at(i);

            entry.set_addr(b.buf);
            entry.set_byte_count(b.data_size);
            entry.set_last_entry(is_last);

            rem -= cur_sectors;

            if is_last {
                break;
            }
        }

        sectors - rem
    }
}

#[repr(C, packed)]
struct PrdEntry {
    addr: u32,
    cnt: u32,
}

impl PrdEntry {
    pub fn set_addr(&mut self, addr: PhysAddr) {
        self.addr = addr.0 as u32;
    }

    pub fn set_byte_count(&mut self, bytes: usize) {
        //assert!(bytes < 0xFFFF && bytes % 2 == 0);
        unsafe {
            self.cnt.set_bits(0..16, bytes as u32);
        }
    }

    pub fn set_last_entry(&mut self, last: bool) {
        unsafe {
            self.cnt.set_bit(31, last);
        }
    }
}

impl IdeChannelData {
    pub fn new(base: u16, ctrl: u16, bmide: u16, interrupt_nr: usize) -> IdeChannelData {
        IdeChannelData {
            base: DevBaseReg::new(base),
            ctrl: DevCtrlReg::new(ctrl),
            bmide: BusMasterReg::new(bmide),
            interrupt_nr,
            prdt_addr: PhysAddr(0),
            active_cmd: None,
        }
    }

    pub fn software_reset(&mut self) {
        self.ctrl.software_reset();
    }

    pub fn setup_prdt(&mut self) {
        let prdt = allocate_order(1).unwrap();

        self.bmide.load_prdt(prdt.address());

        self.prdt_addr = prdt.address();
    }

    pub fn get_prdt(&mut self) -> PrdTable {
        PrdTable::new(self.prdt_addr, 8192 / 8)
    }

    pub fn enable_interrupts(&mut self) {
        set_irq_dest(self.interrupt_nr as u8, self.interrupt_nr as u8 + 32);
        set_active_high(self.interrupt_nr as u8, true);
        add_shared_irq_handler(self.interrupt_nr as usize + 32, ata_handler);

        self.ctrl.enable_interrupts();
    }

    pub fn init(&mut self) {
        self.enable_interrupts();
        self.setup_prdt();

        let _status = self.bmide.status();
    }

    pub fn run_ata_command(
        &mut self,
        cmd: AtaCommand,
        sector: usize,
        count: usize,
        buf: &[DmaBuf],
        slave: bool,
    ) -> usize {
        self.base.clear_features();

        let mut table = self.get_prdt();

        let count = table.load_dma(buf, count);

        let is_lba48 = cmd.is_lba48();

        self.base.set_drive_select(
            slave,
            true,
            if is_lba48 {
                0
            } else {
                sector.get_bits(24..28) as u16
            },
        );
        self.base.set_sector_count(is_lba48, count as u16);
        self.base.set_sector_num(is_lba48, sector);
        self.base.set_command(cmd);

        self.bmide.ack_interrupt();

        self.bmide.start_dma(cmd);

        count
    }

    pub fn run_request(&mut self, request: Arc<DmaRequest>, offset: usize, slave: bool) -> usize {
        self.active_cmd = Some(request.clone());

        let rem = request.count() - offset;

        let max_cnt = 256; // Ask max for 256 sectors at a time

        let cnt = core::cmp::min(rem, max_cnt);

        let cnt = self.run_ata_command(
            request.ata_command(),
            request.sector() + offset,
            cnt,
            request.dma_vec_from(offset),
            slave,
        );

        request.inc_incomplete();

        offset + cnt
    }

    pub fn handle_interrupt(&mut self) {
        if let Some(active) = &self.active_cmd {
            let status = self.bmide.status();

            if !status.contains(BMIdeStatus::DMA_ACTIVE) && status.contains(BMIdeStatus::DISK_IRQ) {
                self.bmide.stop_dma();

                active.dec_incomplete();

                if active.is_complete() {
                    active.wait_queue().notify_one();
                }

                self.active_cmd = None;
            }
        }

        self.bmide.ack_interrupt();
    }

    pub fn detect(&mut self, slave: bool) -> bool {
        self.software_reset();

        let mut sel = BaseDriveSelReg::new();
        sel.set_slave(slave);

        self.base.set_drive_select(slave, false, 0);
        busy_sleep(1000);

        self.base.set_command(AtaCommand::AtaCommandIdentifyDevice);
        busy_sleep(1000);

        let status = self.base.status();

        if status == BaseStatusReg::empty() {
            //println!("No dev");
            return false;
        }

        loop {
            if let Some(status) = self.base.try_status() {
                if status.contains(BaseStatusReg::ERR) {
                    //println!("Err, dev not ata");
                    return false;
                }
                if !status.contains(BaseStatusReg::BSY) && status.contains(BaseStatusReg::DRQ) {
                    break;
                }
            } else {
                //println!("Invalid status");
                return false;
            }
        }

        let lm = self.base.lba_mid();
        let lh = self.base.lba_hi();

        match (lm, lh) {
            //(0x14, 0xEB) => {
            //    //println!("ATADEV_PATAPI");
            //}
            //(0x69, 0x96) => {
            //    //println!("ATADEV_SATAPI");
            //}
            (0x0, 0x0) => {
                // Support only PATA for now
                //println!("ATADEV_PATA");
                return true;
            }
            //(0x3c, 0xc3) => {
            //    //println!("ATADEV_SATA");
            //}
            _ => {
                //println!("ATADEV_UNKNOWN {:#x} {:#x}", a, b)
            }
        }

        return false;
    }
}

pub struct IdeChannel {
    data: Spin<IdeChannelData>,
    cmd_wq: WaitQueue,
}

impl IdeChannel {
    pub fn new(base: u16, ctrl: u16, bmide: u16, interrupt_nr: usize) -> Arc<IdeChannel> {
        Arc::new(IdeChannel {
            data: Spin::new(IdeChannelData::new(base, ctrl, bmide, interrupt_nr)),
            cmd_wq: WaitQueue::new(),
        })
    }

    pub fn init(&self) {
        self.data.lock_irq().init();
    }

    pub fn handle_interrupt(&self) {
        let mut data = self.data.lock_irq();

        data.handle_interrupt();

        if data.active_cmd.is_none() {
            self.cmd_wq.notify_one();
        }
    }

    pub fn detect(&self, slave: bool) -> bool {
        self.data.lock_irq().detect(slave)
    }

    pub fn run_request(&self, request: Arc<DmaRequest>, slave: bool) -> Option<usize> {
        let mut offset = 0;

        while offset < request.count() {
            let data = self
                .cmd_wq
                .wait_lock_irq_for(&self.data, |l| l.active_cmd.is_none());

            match data {
                Ok(mut l) => {
                    offset = l.run_request(request.clone(), offset, slave);
                }
                Err(_e) => {
                    return Some(offset * 512);
                }
            }
        }

        while let Err(_e) = request.wait_queue().wait_for(|| request.is_complete()) {
            // TODO: Make some waits uninterruptible
            //println!("[ IDE ] IO interrupted, retrying");
        }

        Some(request.count() * 512)
    }
}
