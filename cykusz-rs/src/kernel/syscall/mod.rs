pub mod sys;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

#[allow(unused_variables)]
pub fn syscall_handler(num: u64, a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> isize {
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
        SYS_EXIT => sys::sys_exit(),

        _ => Err(SyscallError::Inval),
    }
    .syscall_into()
}
