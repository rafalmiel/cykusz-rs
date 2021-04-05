pub mod sys;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

fn conditional_enable_int(sys: usize) {
    use syscall_defs::*;
    match sys {
        SYS_FUTEX_WAKE | SYS_FUTEX_WAIT => {
            return;
        }
        _ => {
            crate::kernel::int::enable();
        }
    }
}

#[allow(unused_variables)]
pub fn syscall_handler(num: u64, a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> isize {
    conditional_enable_int(num as usize);

    use syscall_defs::*;
    match num as usize {
        SYS_READ => sys::sys_read(a, b, c),
        SYS_WRITE => sys::sys_write(a, b, c),
        SYS_OPEN => sys::sys_open(a, b, c),
        SYS_CLOSE => sys::sys_close(a),
        SYS_CHDIR => sys::sys_chdir(a, b),
        SYS_GETCWD => sys::sys_getcwd(a, b),
        SYS_MKDIR => sys::sys_mkdir(a, b),
        SYS_GETDENTS => sys::sys_getdents(a, b, c),
        SYS_GETADDRINFO => sys::sys_getaddrinfo(a, b, c, d),
        SYS_EXIT => sys::sys_exit(),
        SYS_SLEEP => sys::sys_sleep(a),
        SYS_POWEROFF => sys::sys_poweroff(),
        SYS_REBOOT => sys::sys_reboot(),
        SYS_BIND => sys::sys_bind(a, b),
        SYS_CONNECT => sys::sys_connect(a, b, c, d),
        SYS_SELECT => sys::sys_select(a, b),
        SYS_MOUNT => sys::sys_mount(a, b, c, d, e, f),
        SYS_UMOUNT => sys::sys_umount(a, b),
        SYS_TIME => sys::sys_time(),
        SYS_SYMLINK => sys::sys_symlink(a, b, c, d),
        SYS_RMDIR => sys::sys_rmdir(a, b),
        SYS_UNLINK => sys::sys_unlink(a, b),
        SYS_LINK => sys::sys_link(a, b, c, d),
        SYS_RENAME => sys::sys_rename(a, b, c, d),
        SYS_FORK => sys::sys_fork(),
        SYS_EXEC => sys::sys_exec(a, b, c, d, e, f),
        SYS_FCNTL => sys::sys_fcntl(a, b),
        SYS_MMAP => sys::sys_mmap(a, b, c, d, e, f),
        SYS_MUNMAP => sys::sys_munmap(a, b),
        SYS_MAPS => sys::sys_maps(),
        SYS_WAITPID => sys::sys_waitpid(a),
        SYS_IOCTL => sys::sys_ioctl(a, b, c),
        SYS_SIGACTION => sys::sys_sigaction(a, b, c, d),
        SYS_FUTEX_WAIT => sys::sys_futex_wait(a, b),
        SYS_FUTEX_WAKE => sys::sys_futex_wake(a),
        SYS_ARCH_PRCTL => crate::arch::syscall::sys_arch_prctl(a, b),
        SYS_SPAWN_THREAD => sys::sys_spawn_thread(a, b),

        _ => Err(SyscallError::EINVAL),
    }
    .syscall_into()
}
