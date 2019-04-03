use core::sync::atomic::Ordering;

const WORK_COUNT: usize = 0x5000;
const ITERS: usize = 1; //<usize>::max_value();

pub fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..WORK_COUNT {
        unsafe {
            let _ = a.read_volatile();
        }
    }
}

fn task() {
    for _ in 0..ITERS {
        println!(
            "K( PID: {:<6} CPU: {:<6} MEM: {:<8} LEN: {:<6}),",
            crate::kernel::sched::current_id(),
            unsafe { crate::CPU_ID },
            crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst),
            crate::kernel::sched::queue_len()
        );

        dummy_work();
    }
    crate::kernel::sched::create_task(task);
}

pub fn start() {
    crate::kernel::sched::create_task(task);
    crate::kernel::sched::create_task(task);
    crate::kernel::sched::create_task(task);
    crate::kernel::sched::create_user_task(
        crate::kernel::user::get_user_program(),
        crate::kernel::user::get_user_program_size(),
        0x60000,
    );
}
