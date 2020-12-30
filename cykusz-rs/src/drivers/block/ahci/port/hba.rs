use crate::arch::mm::phys::allocate_order;
use crate::arch::mm::virt::map_to_flags;
use crate::drivers::block::ahci::reg::*;
use crate::drivers::block::ahci::request::DmaBuf;
use crate::kernel::mm::virt::PageFlags;

impl HbaPort {
    pub fn read(&mut self, sector: usize, count: usize, buf: &[DmaBuf], slot: usize) -> bool {
        let hdr = self.cmd_header_at(slot);

        let mut flags = hdr.flags();
        flags.remove(HbaCmdHeaderFlags::W);
        flags.insert(HbaCmdHeaderFlags::P | HbaCmdHeaderFlags::C);
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
        fis.reset();

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

    pub fn probe(&mut self, num: usize) -> bool {
        let sts = self.ssts();

        let ipm = sts.interface_power_management();
        let dev = sts.device_detection();

        if let (HbaPortSstsRegDet::PresentAndE, HbaPortSstsRegIpm::Active) = (dev, ipm) {
            println!("[ AHCI ] Enabling Ahci port {}", num);

            self.start();

            return true;
        }

        false
    }
}
