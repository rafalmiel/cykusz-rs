use core::sync::atomic::Ordering;

use crate::kernel::sched::current_task;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

pub fn syscall_handler(num: u64, a: u64, b: u64, c: u64) -> u64 {
    match num {
        0 => {
            let fd = a as usize;

            let task = current_task();
            if let Some(f) = task.get_handle(fd) {
                let ptr = b as *const u8;
                let buf = unsafe { core::slice::from_raw_parts(ptr, c as usize) };

                return f.inode.write_at(0, buf).unwrap_or(0) as u64;
            }

            return 0;
        }
        1 => {
            let fd = a as usize;

            let task = current_task();
            let n = if let Some(f) = task.get_handle(fd) {
                let buf = unsafe { core::slice::from_raw_parts_mut(b as *mut u8, c as usize) };
                f.inode.read_at(0, buf).unwrap_or(0)
            } else {
                0
            };

            return n as u64;
        }
        2 => {
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
