use core::sync::atomic::Ordering;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn syscall_handler(_num: u32) {
    println!(
        "S( PID: {:<6} CPU: {:<6} MEM: {:<8}),",
        crate::kernel::sched::current_id(),
        unsafe { crate::CPU_ID },
        crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst)
    );
}
