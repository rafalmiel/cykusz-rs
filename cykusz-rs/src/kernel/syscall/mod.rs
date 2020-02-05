
pub mod sys;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

#[allow(unused_variables)]
pub fn syscall_handler(num: u64, a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> u64 {
    use syscall_defs::*;
    //syscall_defs::user::hello();
    match num as usize {
        SYS_READ => sys::sys_read(a, b, c),
        SYS_WRITE => sys::sys_write(a, b, c),
        SYS_OPEN => sys::sys_open(a, b, c),
        SYS_CLOSE => sys::sys_close(a),

        _ => {
            return 0;
        }
    }

}
