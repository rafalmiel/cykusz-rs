use core::ptr;

use kernel::mm::{PhysAddr,MappedAddr};

pub fn init() {
    extern {
        static apinit_start: u8;
        static apinit_end: u8;
    }

    unsafe {
        let start = &apinit_start as *const _ as usize;
        let end = &apinit_end as *const _ as usize;

        let p = PhysAddr(&apinit_start as *const _ as usize).to_mapped().0 as *const u8;

        p.copy_to(PhysAddr(0x1000).to_mapped().0 as *mut u8, end - start);
    }

    ::arch::dev::lapic::init_ap();
}