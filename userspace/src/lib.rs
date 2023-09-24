#![no_std]
#![allow(internal_features)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]
#![feature(linkage)]

extern crate alloc;
extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;
extern crate user_alloc;

use core::panic::PanicInfo;

use syscall_defs::{FcntlCmd, OpenFlags, SyscallError};

#[allow(unused)]
pub fn bochs() {
    unsafe {
        asm!("xchg bx, bx");
    }
}

#[linkage = "weak"]
#[no_mangle]
pub fn main() {}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    /* fd 0 stdin */
    if let Err(SyscallError::EBADFD) = syscall::fcntl(0, FcntlCmd::GetFL) {
        syscall::open("/dev/tty", OpenFlags::RDONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stdin");
    }
    /* fd 1 stdout */
    if let Err(SyscallError::EBADFD) = syscall::fcntl(1, FcntlCmd::GetFL) {
        syscall::open("/dev/tty", OpenFlags::WRONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stdout");
    }

    /* fd 2 stderr*/
    if let Err(SyscallError::EBADFD) = syscall::fcntl(2, FcntlCmd::GetFL) {
        syscall::open("/dev/tty", OpenFlags::WRONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stderr");
    }

    user_alloc::init();

    main();

    syscall::exit(0);
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panicked... {:?}", info);
    syscall::exit(1)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[alloc_error_handler]
fn oom(layout: core::alloc::Layout) -> ! {
    println!("Out of memory! {:?}", layout);
    syscall::exit(1)
}
