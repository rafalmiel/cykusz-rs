use crate::arch::mm::VirtAddr;
use core::arch::asm;
use core::panic::PanicInfo;

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[lang = "panic_impl"]
pub fn panic_impl(pi: &PanicInfo) -> ! {
    println!("{} PANIC: {:?}", crate::cpu_id(), pi);
    logln!("PANIC: {:?}", pi);
    dbgln!(sched_v, "panic");
    print_current_backtrace();
    loop {}
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

pub fn print_current_backtrace() {
    let mut rip: usize;
    let mut rbp: usize;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }

    while !VirtAddr(rbp).is_user() {
        rip = unsafe { VirtAddr(rbp + 8).read::<usize>() };

        dbgln!(sched_v, "{:#x}", rip);

        rbp = unsafe { VirtAddr(rbp).read::<usize>() }
    }
}
