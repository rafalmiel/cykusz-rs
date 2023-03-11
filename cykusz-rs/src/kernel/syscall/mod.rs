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
        SYS_FUTEX_WAKE | SYS_FUTEX_WAIT | SYS_EXIT | SYS_EXIT_THREAD => {
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
        SYS_OPEN => sys::sys_open(a, b, c, d),
        SYS_CLOSE => sys::sys_close(a),
        SYS_CHDIR => sys::sys_chdir(a, b),
        SYS_GETCWD => sys::sys_getcwd(a, b),
        SYS_MKDIR => sys::sys_mkdir(a, b),
        SYS_GETDENTS => sys::sys_getdents(a, b, c),
        SYS_GETADDRINFO => sys::sys_getaddrinfo(a, b, c, d),
        SYS_EXIT => sys::sys_exit(a),
        SYS_SLEEP => sys::sys_sleep(a),
        SYS_POWEROFF => sys::sys_poweroff(),
        SYS_REBOOT => sys::sys_reboot(),
        SYS_BIND => sys::sys_bind(a, b),
        SYS_CONNECT => sys::sys_connect(a, b, c, d),
        SYS_SELECT => sys::sys_select(a, b, c, d, e, f),
        SYS_MOUNT => sys::sys_mount(a, b, c, d, e, f),
        SYS_UMOUNT => sys::sys_umount(a, b),
        SYS_TIME => sys::sys_time(),
        SYS_SYMLINK => sys::sys_symlink(a, b, c, d),
        SYS_RMDIR => sys::sys_rmdir(a, b),
        SYS_UNLINK => sys::sys_unlink(a, b, c, d),
        SYS_LINK => sys::sys_link(a, b, c, d),
        SYS_RENAME => sys::sys_rename(a, b, c, d),
        SYS_FORK => sys::sys_fork(),
        SYS_EXEC => sys::sys_exec(a, b, c, d, e, f),
        SYS_FCNTL => sys::sys_fcntl(a, b),
        SYS_MMAP => sys::sys_mmap(a, b, c, d, e, f),
        SYS_MUNMAP => sys::sys_munmap(a, b),
        SYS_MAPS => sys::sys_maps(),
        SYS_SEEK => sys::sys_seek(a, b, c),
        SYS_PREAD => sys::sys_pread(a, b, c, d),
        SYS_PWRITE => sys::sys_pwrite(a, b, c, d),
        SYS_WAITPID => sys::sys_waitpid(a, b, c),
        SYS_IOCTL => sys::sys_ioctl(a, b, c),
        SYS_SIGACTION => sys::sys_sigaction(a, b, c, d),
        SYS_SIGPROCMASK => sys::sys_sigprocmask(a, b, c),
        SYS_FUTEX_WAIT => sys::sys_futex_wait(a, b),
        SYS_FUTEX_WAKE => sys::sys_futex_wake(a),
        SYS_ARCH_PRCTL => crate::arch::syscall::sys_arch_prctl(a, b),
        SYS_SPAWN_THREAD => sys::sys_spawn_thread(a, b),
        SYS_EXIT_THREAD => sys::sys_exit_thread(),
        SYS_GETPID => sys::sys_getpid(),
        SYS_GETTID => sys::sys_gettid(),
        SYS_SETSID => sys::sys_setsid(),
        SYS_SETPGID => sys::sys_setpgid(a, b),
        SYS_PIPE => sys::sys_pipe(a, b),
        SYS_DUP => sys::sys_dup(a, b),
        SYS_DUP2 => sys::sys_dup2(a, b, c),
        SYS_STAT => sys::sys_stat(a, b, c),
        SYS_FSTAT => sys::sys_fstat(a, b),
        SYS_GETRLIMIT => sys::sys_getrlimit(a, b),
        SYS_DEBUG => sys::sys_debug(a, b),
        SYS_ACCESS => sys::sys_access(a, b, c, d, e),
        SYS_KILL => sys::sys_kill(a, b),
        SYS_SYNC => sys::sys_sync(),
        SYS_FSYNC => sys::sys_fsync(a),
        SYS_TICKSNS => sys::sys_ticksns(),
        a => {
            logln!("NO SYS????? {}", a);
            Err(SyscallError::ENOSYS)
        },
    }
    .syscall_into()
}
