#![no_std]
#![feature(llvm_asm)]

use syscall_defs::*;

#[macro_use]
pub mod print;

pub unsafe fn syscall0(mut a: usize) -> SyscallResult {
    llvm_asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub unsafe fn syscall1(mut a: usize, b: usize) -> SyscallResult {
    llvm_asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub unsafe fn syscall2(mut a: usize, b: usize, c: usize) -> SyscallResult {
    llvm_asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub unsafe fn syscall3(mut a: usize, b: usize, c: usize, d: usize) -> SyscallResult {
    llvm_asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub unsafe fn syscall4(mut a: usize, b: usize, c: usize, d: usize, e: usize) -> SyscallResult {
    llvm_asm!("syscall"
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
    llvm_asm!("syscall"
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
    llvm_asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d), "{r10}"(e), "{r8}"(f), "{r9}"(g)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    SyscallResult::syscall_from(a as isize)
}

pub fn read(fd: usize, buf: &mut [u8]) -> SyscallResult {
    unsafe { syscall3(SYS_READ, fd, buf.as_mut_ptr() as usize, buf.len()) }
}

pub fn write(fd: usize, buf: &[u8]) -> SyscallResult {
    unsafe { syscall3(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len()) }
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

pub fn getcwd(buf: &mut [u8]) -> SyscallResult {
    unsafe { syscall2(SYS_GETCWD, buf.as_mut_ptr() as usize, buf.len()) }
}

pub fn mkdir(path: &str) -> SyscallResult {
    unsafe { syscall2(SYS_MKDIR, path.as_ptr() as usize, path.len()) }
}

pub fn bind(port: u32, flags: syscall_defs::ConnectionFlags) -> SyscallResult {
    unsafe { syscall2(SYS_BIND, port as usize, flags.bits()) }
}

pub fn connect(host: &[u8], port: u32, flags: syscall_defs::ConnectionFlags) -> SyscallResult {
    unsafe {
        syscall4(
            SYS_CONNECT,
            host.as_ptr() as usize,
            host.len(),
            port as usize,
            flags.bits(),
        )
    }
}

pub fn select(fds: &[u8]) -> SyscallResult {
    unsafe { syscall2(SYS_SELECT, fds.as_ptr() as usize, fds.len()) }
}

pub fn getdents(fd: usize, buf: &mut [u8]) -> SyscallResult {
    unsafe {
        syscall3(
            SYS_GETDENTS,
            fd as usize,
            buf.as_mut_ptr() as usize as usize,
            buf.len(),
        )
    }
}

pub fn getaddrinfo(name: &str, buf: &mut [u8]) -> SyscallResult {
    unsafe {
        syscall4(
            SYS_GETADDRINFO,
            name.as_ptr() as usize,
            name.len(),
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    }
}

pub fn mount(dev: &str, dest: &str, fs: &str) -> SyscallResult {
    unsafe {
        syscall6(
            SYS_MOUNT,
            dev.as_ptr() as usize,
            dev.len(),
            dest.as_ptr() as usize,
            dest.len(),
            fs.as_ptr() as usize,
            fs.len(),
        )
    }
}

pub fn time() -> Result<isize, SyscallError> {
    unsafe {
        let res = syscall0(SYS_TIME);

        match res {
            Ok(t) => Ok(t as isize),
            Err(e) => Err(e),
        }
    }
}

pub fn umount(path: &str) -> SyscallResult {
    unsafe { syscall2(SYS_UMOUNT, path.as_ptr() as usize, path.len()) }
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
    write(0, v.as_bytes()).unwrap();
}
