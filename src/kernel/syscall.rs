use core::sync::atomic::Ordering;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

pub fn syscall_handler(_num: u32) {
    println!(
        "U( {:<6} PID: {:<6} CPU: {:<6} MEM: {:<8}{:<12}),",
        "",
        crate::kernel::sched::current_id(),
        unsafe { crate::CPU_ID },
        crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst),
        ""
    );
}
