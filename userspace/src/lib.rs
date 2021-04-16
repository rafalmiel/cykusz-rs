#![no_std]
#![feature(llvm_asm)]
#![feature(lang_items)]
#![feature(extended_key_value_attributes)]
#![feature(linkage)]

extern crate alloc;
extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;
extern crate user_alloc;

use core::alloc::Layout;
use core::panic::PanicInfo;

use syscall_defs::{FcntlCmd, OpenFlags, SyscallError};

#[allow(unused)]
pub fn bochs() {
    unsafe {
        llvm_asm!("xchg %bx, %bx");
    }
}

#[linkage = "weak"]
#[no_mangle]
pub fn main() {}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    /* fd 0 stdin */
    if syscall::fcntl(0, FcntlCmd::GetFL) == Err(SyscallError::EBADFD) {
        syscall::open("/dev/tty", OpenFlags::RDONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stdin");
    }
    /* fd 1 stdout */
    if syscall::fcntl(1, FcntlCmd::GetFL) == Err(SyscallError::EBADFD) {
        syscall::open("/dev/tty", OpenFlags::WRONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stdout");
    }

    /* fd 2 stderr*/
    if syscall::fcntl(2, FcntlCmd::GetFL) == Err(SyscallError::EBADFD) {
        syscall::open("/dev/tty", OpenFlags::WRONLY | OpenFlags::NOCTTY)
            .expect("Failed to open /dev/stdout");
    }

    user_alloc::init();

    main();

    syscall::exit();
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panicked... {:?}", info);
    syscall::exit()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[lang = "oom"]
fn oom(layout: Layout) -> ! {
    println!("Out of memory! {:?}", layout);
    syscall::exit()
}
