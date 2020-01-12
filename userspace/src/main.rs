#![no_std]
#![no_main]

#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;

use core::panic::PanicInfo;

#[allow(unused)]
macro_rules! int {
    ( $x:expr) => {
        {
            asm!("int $0" :: "N"($x));
        }
    };
}

pub unsafe fn syscall0(mut a: usize) -> usize {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a)
        : "rcx", "r11", "memory"
        : "intel", "volatile");

    a
}

#[allow(unused)]
pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

const WORK_COUNT: usize = 0x5000000;

pub fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..WORK_COUNT {
        unsafe {
            let _ = a.read_volatile();
        }
    }
}
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    loop {
        dummy_work();

        unsafe {
            syscall0(32);
        }
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
