use core::sync::atomic::Ordering;

pub mod sys;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_OPEN: usize = 2;
const SYS_CLOSE: usize = 3;
const SYS_PRINTDEBUG: usize = 666;

#[allow(unused_variables)]
pub fn syscall_handler(num: u64, a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> u64 {
    //println!("Got syscall: {}", num);
    match num as usize {
        SYS_READ => sys::sys_read(a, b, c),
        SYS_WRITE => sys::sys_write(a, b, c),
        SYS_OPEN => sys::sys_open(a, b, c),
        SYS_CLOSE => sys::sys_close(a),

        SYS_PRINTDEBUG => {
            // print diagnostics
            println!(
                "U( {:<6} PID: {:<6} CPU: {:<6} MEM: {:<8}{:<12}),",
                "",
                crate::kernel::sched::current_id(),
                unsafe { crate::CPU_ID },
                crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst),
                ""
            );

            return 0;
        }
        _ => {
            return 0;
        }
    }
}
