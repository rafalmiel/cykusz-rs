#![no_std]
#![no_main]

#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;

use core::panic::PanicInfo;

macro_rules! int {
    ( $x:expr) => {
        {
            asm!("int $0" :: "N"($x));
        }
    };
}

const WORK_COUNT: usize = 0x5000000;
const ITERS: usize = 1; //<usize>::max_value();

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
            int!(80);
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
