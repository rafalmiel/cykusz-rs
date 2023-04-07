use crate::arch::mm::virt::map_to_flags;

use crate::drivers::block::ahci::reg::*;
use crate::drivers::block::ata::request::DmaBuf;
use crate::drivers::block::ata::AtaCommand;

use crate::kernel::mm::allocate_order;
use crate::kernel::mm::virt::PageFlags;

impl HbaPort {
    pub fn run_command(
        &mut self,
        cmd: AtaCommand,
        sector: usize,
        count: usize,
        buf: &[DmaBuf],
        slot: usize,
    ) -> bool {
        let hdr = self.cmd_header_at(slot);

        let mut flags = HbaCmdHeaderFlags::empty();
        if cmd == AtaCommand::AtaCommandWriteDmaExt || cmd == AtaCommand::AtaCommandWriteDma {
            flags.insert(HbaCmdHeaderFlags::W);
        } else {
            flags.remove(HbaCmdHeaderFlags::W);
        }
        //flags.insert(HbaCmdHeaderFlags::C);
        flags.set_command_fis_length((core::mem::size_of::<FisRegH2D>() / 4) as u8);

        hdr.set_flags(flags);

        let l = ((count - 1) >> 4) + 1;
        hdr.set_prdtl(l);
        hdr.set_prd_byte_count(0);

        let tbl = hdr.cmd_tbl();
        tbl.reset();

        for pri in 0..l {
            let prdt = tbl.prdt_entry_mut(pri);
            prdt.reset();

            prdt.set_database_address(buf[pri].buf);
            prdt.set_data_byte_count(buf[pri].data_size - 1);
            prdt.set_interrupt_on_completion(false);
        }

        let fis = tbl.cfis_as_h2d_mut();
        fis.reset();

        fis.set_fis_type(FisType::RegH2D);
        fis.set_c(true);
        fis.set_command(cmd);
        fis.set_lba(sector, 1 << 6);

        fis.set_count(count as u16);

        self.set_ci(1 << slot); // issue cmd

        true
    }

    pub fn start(&mut self) {
        self.stop_cmd();

        let addr = allocate_order(1).unwrap().address();

        for o in (0..0x2000).step_by(0x1000) {
            map_to_flags(
                addr.to_virt() + o,
                addr + o,
                PageFlags::WRITABLE | PageFlags::NO_CACHE,
            );
        }
        unsafe {
            addr.to_virt().as_bytes_mut(8092).fill(0);
        }

        let cba = allocate_order(0).unwrap().address();
        map_to_flags(
            cba.to_virt(),
            cba,
            PageFlags::WRITABLE | PageFlags::NO_CACHE,
        );
        unsafe {
            cba.to_virt().as_bytes_mut(4096).fill(0);
        }

        self.set_clb(cba);
        self.set_fb(cba + 1024);

        for i in 0..32 {
            let cmd_hdr = self.cmd_header_at(i);

            cmd_hdr.set_prdtl(8);
            cmd_hdr.set_prd_byte_count(0);
            cmd_hdr.set_cmd_tbl_base_addr(addr + 256 * i);
        }

        self.set_ie(HbaPortIEReg::DHRE | HbaPortIEReg::INFE);

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
