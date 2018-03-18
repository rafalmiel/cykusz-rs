use core::ptr;

use kernel::mm::{PhysAddr,MappedAddr};

pub const TRAMPOLINE : PhysAddr = PhysAddr(0xE00);
pub const AP_INIT : PhysAddr = PhysAddr(0x1000);

pub fn init() {
    extern {
        static apinit_start: u8;
        static apinit_end: u8;
        static trampoline: u8;
    }

    // Copy over trampoline and apinit code to 0xE00 and 0x1000
    unsafe {
        let start = &apinit_start as *const _ as usize;
        let end = &apinit_end as *const _ as usize;

        let tramp = &trampoline as *const _ as usize;

        let p = PhysAddr(start).to_mapped().0 as *const u8;

        p.copy_to(AP_INIT.to_mapped().0 as *mut u8, end - start);

        let pt = PhysAddr(tramp).to_mapped().0 as *const u8;

        pt.copy_to(TRAMPOLINE.to_mapped().0 as *mut u8, 0x100);
    }

    ::arch::dev::lapic::init_ap();
}