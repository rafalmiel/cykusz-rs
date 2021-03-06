#![no_std]
#![feature(llvm_asm)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::AtomicU32;

use syscall_defs::*;
use syscall_defs::signal::SigAction;

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

pub fn debug(str: &str) -> SyscallResult {
    unsafe { syscall2(SYS_DEBUG, str.as_ptr() as usize, str.len() as usize) }
}

pub fn open(path: &str, flags: syscall_defs::OpenFlags) -> SyscallResult {
    let fd = OpenFD::Cwd;
    unsafe {
        syscall4(
            SYS_OPEN,
            fd.into(),
            path.as_ptr() as usize,
            path.len(),
            flags.bits(),
        )
    }
}

pub fn access(path: &str) -> SyscallResult {
    let fd = OpenFD::Cwd;

    unsafe {
        syscall5(
            SYS_ACCESS,
            fd.into(),
            path.as_ptr() as usize,
            path.len(),
            0,
            0,
        )
    }
}

pub fn close(fd: usize) -> SyscallResult {
    unsafe { syscall1(SYS_CLOSE, fd) }
}

fn _fcntl(fd: usize, cmd: FcntlCmd) -> SyscallResult {
    unsafe { syscall2(SYS_FCNTL, fd, cmd as usize) }
}

pub fn fcntl(fd: usize, cmd: FcntlCmd) -> Result<OpenFlags, SyscallError> {
    let res = _fcntl(fd, cmd);

    match res {
        Ok(r) => {
            if let Some(fl) = OpenFlags::from_bits(r) {
                Ok(fl)
            } else {
                Err(SyscallError::EINVAL)
            }
        }
        Err(e) => Err(e),
    }
}

pub fn mmap(
    addr: Option<usize>,
    size: usize,
    prot: MMapProt,
    flags: MMapFlags,
    fd: Option<usize>,
    offset: usize,
) -> SyscallResult {
    unsafe {
        syscall6(
            SYS_MMAP,
            addr.unwrap_or(0),
            size,
            prot.bits(),
            flags.bits(),
            fd.unwrap_or(usize::MAX),
            offset,
        )
    }
}

pub fn munmap(addr: usize, len: usize) -> SyscallResult {
    unsafe { syscall2(SYS_MUNMAP, addr, len) }
}

pub fn maps() -> SyscallResult {
    unsafe { syscall0(SYS_MAPS) }
}

pub fn sync() -> SyscallResult {
    unsafe { syscall0(SYS_SYNC) }
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

pub fn symlink(target: &str, path: &str) -> SyscallResult {
    unsafe {
        syscall4(
            SYS_SYMLINK,
            target.as_ptr() as usize,
            target.len(),
            path.as_ptr() as usize,
            path.len(),
        )
    }
}

pub fn link(target: &str, path: &str) -> SyscallResult {
    unsafe {
        syscall4(
            SYS_LINK,
            target.as_ptr() as usize,
            target.len(),
            path.as_ptr() as usize,
            path.len(),
        )
    }
}

pub fn rmdir(path: &str) -> SyscallResult {
    unsafe { syscall2(SYS_RMDIR, path.as_ptr() as usize, path.len()) }
}

pub fn unlink(path: &str) -> SyscallResult {
    let fd = OpenFD::Cwd;
    unsafe { syscall4(SYS_UNLINK, fd.into(), path.as_ptr() as usize, path.len(), 0) }
}

pub fn rename(oldpath: &str, newpath: &str) -> SyscallResult {
    unsafe {
        syscall4(
            SYS_RENAME,
            oldpath.as_ptr() as usize,
            oldpath.len(),
            newpath.as_ptr() as usize,
            newpath.len(),
        )
    }
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

pub fn exit(status: isize) -> ! {
    unsafe {
        syscall1(SYS_EXIT, status as usize).expect("Failed to exit");
    }

    unreachable!()
}

pub fn sleep(time_ms: usize) -> SyscallResult {
    unsafe { syscall1(SYS_SLEEP, time_ms * 1_000_000) }
}

pub fn fork() -> SyscallResult {
    unsafe { syscall0(SYS_FORK) }
}

pub fn exec(path: &str, args: Option<&[&str]>, env: Option<&[&str]>) -> SyscallResult {
    let args = if let Some(args) = args {
        syscall_defs::exec::into_syscall_slice(args)
    } else {
        Vec::new()
    };
    let env = if let Some(env) = env {
        syscall_defs::exec::into_syscall_slice(env)
    } else {
        Vec::new()
    };

    unsafe {
        syscall6(
            SYS_EXEC,
            path.as_ptr() as usize,
            path.len(),
            args.as_ptr() as usize,
            args.len(),
            env.as_ptr() as usize,
            env.len(),
        )
    }
}

pub fn waitpid(pid: usize, status: &mut u32) -> SyscallResult {
    unsafe { syscall3(SYS_WAITPID, pid as usize, status as *const u32 as usize, 0) }
}

pub fn ioctl(fd: usize, cmd: usize, arg: usize) -> SyscallResult {
    unsafe { syscall3(SYS_IOCTL, fd, cmd, arg) }
}

pub fn sigaction(
    sig: usize,
    sigaction: Option<&syscall_defs::signal::SigAction>,
    old_sigaction: Option<&mut syscall_defs::signal::SigAction>,
) -> SyscallResult {
    let sigact = sigaction;

    unsafe {
        syscall4(
            SYS_SIGACTION,
            sig,
            sigact.and_then(|f| {
                Some(f as *const SigAction as usize)
            }).unwrap_or(0),
            sigreturn as usize,
            old_sigaction.and_then(|f| {
                Some(f as *mut SigAction as usize)
            }).unwrap_or(0)
        )
    }
}

pub fn sigprocmask(
    how: syscall_defs::signal::SigProcMask,
    set: &mut u64,
    old_set: Option<&mut u64>,
) -> SyscallResult {
    let set = set as *const u64 as usize;
    let old_set = if let Some(f) = old_set {
        f as *const u64 as usize
    } else {
        0
    };
    unsafe { syscall3(SYS_SIGPROCMASK, how as usize, set, old_set) }
}

#[allow(unused)]
pub fn bochs() {
    unsafe {
        llvm_asm!("xchg %bx, %bx");
    }
}

pub fn sigreturn() -> SyscallResult {
    unsafe { syscall0(SYS_SIGRETURN) }
}

pub fn futex_wait(val: &AtomicU32, expected: u32) -> SyscallResult {
    unsafe {
        syscall2(
            SYS_FUTEX_WAIT,
            (val as *const AtomicU32) as usize,
            expected as usize,
        )
    }
}

pub fn futex_wake(val: &AtomicU32) -> SyscallResult {
    unsafe { syscall1(SYS_FUTEX_WAKE, (val as *const AtomicU32) as usize) }
}

pub fn arch_prctl(cmd: syscall_defs::prctl::PrctlCmd, addr: usize) -> SyscallResult {
    unsafe { syscall2(SYS_ARCH_PRCTL, cmd as usize, addr) }
}

pub fn spawn_thread(entry: fn(usize) -> i32, stack: usize) -> SyscallResult {
    unsafe { syscall2(SYS_SPAWN_THREAD, entry as usize, stack) }
}

pub fn exit_thread() -> SyscallResult {
    unsafe { syscall0(SYS_EXIT_THREAD) }
}

pub fn getpid() -> SyscallResult {
    unsafe { syscall0(SYS_GETPID) }
}

pub fn gettid() -> SyscallResult {
    unsafe { syscall0(SYS_GETTID) }
}

pub fn setsid() -> SyscallResult {
    unsafe { syscall0(SYS_SETSID) }
}

pub fn setpgid(pid: usize, pgid: usize) -> SyscallResult {
    unsafe { syscall2(SYS_SETPGID, pid, pgid) }
}

pub fn pipe(fds: &mut [u64], flags: OpenFlags) -> SyscallResult {
    if fds.len() < 2 {
        return Err(SyscallError::EINVAL);
    }

    unsafe { syscall2(SYS_PIPE, fds.as_ptr() as usize, flags.bits()) }
}

pub fn dup(fd: usize, flags: OpenFlags) -> SyscallResult {
    unsafe { syscall2(SYS_DUP, fd, flags.bits()) }
}

pub fn dup2(fd: usize, new_fd: usize, flags: OpenFlags) -> SyscallResult {
    unsafe { syscall3(SYS_DUP2, fd, new_fd, flags.bits()) }
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
    write(1, v.as_bytes()).unwrap();
}
