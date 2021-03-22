use core::alloc::Layout;
use core::panic::PanicInfo;

use syscall_defs::{FcntlCmd, OpenFlags, SyscallError};

#[allow(unused)]
pub fn bochs() {
    unsafe {
        llvm_asm!("xchg %bx, %bx");
    }
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    /* fd 0 stdin */
    if syscall::fcntl(0, FcntlCmd::GetFL) == Err(SyscallError::BadFD) {
        syscall::open("/dev/stdin", OpenFlags::RDONLY).expect("Failed to open /dev/stdin");
    }
    /* fd 1 stdout */
    if syscall::fcntl(1, FcntlCmd::GetFL) == Err(SyscallError::BadFD) {
        syscall::open("/dev/stdout", OpenFlags::WRONLY).expect("Failed to open /dev/stdout");
    }

    /* fd 2 stderr*/
    if syscall::fcntl(2, FcntlCmd::GetFL) == Err(SyscallError::BadFD) {
        syscall::open("/dev/stdout", OpenFlags::WRONLY).expect("Failed to open /dev/stdout");
    }

    user_alloc::init();

    super::main_cd()
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
