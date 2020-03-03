#![no_std]
#![feature(asm)]

use syscall_defs::*;

#[macro_use]
pub mod print;

pub unsafe fn syscall0(mut a: usize) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall1(mut a: usize, b: usize) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall2(mut a: usize, b: usize, c: usize) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall3(mut a: usize, b: usize, c: usize, d: usize) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall4(mut a: usize, b: usize, c: usize, d: usize, e: usize) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d), "{r10}"(e)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall5(
    mut a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d), "{r10}"(e), "{r8}"(f)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}
pub unsafe fn syscall6(
    mut a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    g: usize,
) -> SyscallResult {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d), "{r10}"(e), "{r8}"(f), "{r9}"(g)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub fn read(fd: usize, buf: *mut u8, len: usize) -> SyscallResult {
    unsafe { syscall3(SYS_READ, fd, buf as usize, len) }
}

pub fn write(fd: usize, buf: *const u8, len: usize) -> SyscallResult {
    unsafe { syscall3(SYS_WRITE, fd, buf as usize, len) }
}

pub fn open(path: &str, flags: syscall_defs::OpenFlags) -> SyscallResult {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as usize, path.len(), flags.bits()) }
}

pub fn close(fd: usize) -> SyscallResult {
    unsafe { syscall1(SYS_CLOSE, fd) }
}

pub fn chdir(path: &str) -> SyscallResult {
    unsafe { syscall2(SYS_CHDIR, path.as_ptr() as usize, path.len()) }
}

pub fn getcwd(buf: *mut u8, len: usize) -> SyscallResult {
    unsafe { syscall2(SYS_GETCWD, buf as usize, len) }
}

pub fn mkdir(path: &str) -> SyscallResult {
    unsafe { syscall2(SYS_MKDIR, path.as_ptr() as usize, path.len()) }
}

pub fn getdents(fd: usize, buf: *mut u8, len: usize) -> SyscallResult {
    unsafe { syscall3(SYS_GETDENTS, fd as usize, buf as usize, len) }
}

pub fn exit() -> ! {
    unsafe {
        syscall0(SYS_EXIT).expect("Failed to exit");
    }

    unreachable!()
}

pub fn sleep(time_ms: usize) -> SyscallResult {
    unsafe { syscall1(SYS_SLEEP, time_ms * 1_000_000) }
}

pub fn poweroff() -> ! {
    unsafe {
        if let Err(e) = syscall0(SYS_POWEROFF) {
            panic!("Power Off failed: {:?}", e);
        }

        unreachable!()
    }
}

pub fn reboot() -> SyscallResult {
    unsafe { syscall0(SYS_REBOOT) }
}

pub fn print(v: &str) {
    write(0, v.as_ptr(), v.len()).unwrap();
}
