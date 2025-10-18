use core::arch::asm;

use crate::kernel::mm::PhysAddr;
use crate::kernel::sync::LockApi;

pub const TRAMPOLINE: PhysAddr = PhysAddr(0xE00);
pub const AP_INIT: PhysAddr = PhysAddr(0x1000);

#[repr(C, packed)]
pub struct Trampoline {
    pub ready: u8,
    pub cpu_num: u8,
    pub stack_ptr: u64,
    pub page_table_ptr: u64,
}

impl Trampoline {
    pub fn get() -> &'static mut Trampoline {
        unsafe { TRAMPOLINE.to_mapped().read_mut::<Trampoline>() }
    }

    pub fn reset(&mut self) {
        self.ready = 0;
        self.cpu_num = 0;
        self.stack_ptr = 0;
        self.page_table_ptr = 0;
    }

    pub fn notify_ready(&mut self) {
        let rdy = &mut self.ready as *mut u8;

        unsafe {
            rdy.write_volatile(1);
        }
    }

    pub fn wait_ready(&self) {
        let rdy = &self.ready as *const u8;

        unsafe {
            while rdy.read_volatile() == 0 {
                asm!("pause");
            }
        }
    }
}

static mut CPU_COUNT: usize = 0;

pub fn cpu_count() -> usize {
    unsafe { CPU_COUNT }
}

pub fn init() {
    unsafe {
        CPU_COUNT = crate::arch::acpi::ACPI
            .lock()
            .get_apic_entry()
            .unwrap()
            .lapic_entries()
            .filter(|e| e.proc_is_enabled())
            .count();
    }
}

pub fn start() {
    unsafe extern "C" {
        static apinit_start: u8;
        static apinit_end: u8;
        static trampoline: u8;
    }

    // Copy over trampoline and apinit code to 0xE00 and 0x1000
    unsafe {
        let start = &apinit_start as *const _ as usize;
        let end = &apinit_end as *const _ as usize;

        let tramp = &trampoline as *const _ as usize;

        let p = PhysAddr(start).to_mapped();

        p.copy_to(AP_INIT.to_mapped().0, end - start);

        let pt = PhysAddr(tramp).to_mapped();

        pt.copy_to(TRAMPOLINE.to_mapped().0, 0x100);
    }

    crate::arch::dev::lapic::start_ap();
}

pub fn notify_ap_ready() {
    let trampoline = crate::arch::smp::Trampoline::get();

    trampoline.notify_ready();
}
