#![no_std]
#![no_main]

#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;

#[macro_use]
pub mod print;
pub mod syscall;

use core::str;
use core::panic::PanicInfo;

#[allow(unused)]
pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

const WORK_COUNT: usize = 0x50;

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
        let mut buf = [0u8; 256];

        let r = syscall::read(buf.as_mut_ptr(), buf.len());

        let s = &buf[..r];

        println!("Got {}, bytes: {}", str::from_utf8(s).unwrap(), r);

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
