use core::sync::atomic::Ordering;

pub fn init() {
    ::arch::syscall::init();
}

pub fn syscall_handler(_num: u32) {
    println!("S( PID: {:<6} CPU: {:<6} MEM: {:<8}),",
             ::kernel::sched::current_id(),
             unsafe {::CPU_ID },
             ::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst));
}
