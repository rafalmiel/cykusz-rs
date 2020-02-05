#![no_std]
#![no_main]
#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;
#[macro_use]
extern crate syscall_user as syscall;

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

        print!("[root /]# ");
        let r = syscall::read(1, buf.as_mut_ptr(), buf.len());

        println!("[open: /dev/test_file]");
        let mut fd = syscall::open("/dev/test_file", false);
        println!("[write: {} bytes to fd = {}]", r, fd);
        syscall::write(fd, buf.as_ptr(), r);
        println!("[close: fd = {}]", fd);
        syscall::close(fd);

        let mut buf2 = [0u8; 256];
        println!("[open: /dev/test_file]");
        fd = syscall::open("/dev/test_file", true);
        println!("[read: fd = {}]", fd);
        let read = syscall::read(fd, buf2.as_mut_ptr(), buf2.len());
        println!("[close: fd = {}]", fd);
        syscall::close(fd);

        let s = core::str::from_utf8(&buf2[0..read]).unwrap().trim_end_matches("\n");

        println!("[got: {} bytes: {}]", read, s);
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panicked... {:?}", info);
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
