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
    /* fd 0 */
    if syscall::fcntl(0, FcntlCmd::GetFL) == Err(SyscallError::BadFD) {
        syscall::open("/dev/stdout", OpenFlags::WRONLY).expect("Failed to open /dev/stdout");
    }

    /* fd 1 */
    if syscall::fcntl(1, FcntlCmd::GetFL) == Err(SyscallError::BadFD) {
        syscall::open("/dev/stdin", OpenFlags::RDONLY).expect("Failed to open /dev/stdin");
    }

    super::main_cd()
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
