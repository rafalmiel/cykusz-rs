use core::sync::atomic::Ordering;

const WORK_COUNT: usize = 0x5000;
const ITERS: usize = 1;//<usize>::max_value();

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
        println!("K( PID: {:<6} CPU: {:<6} MEM: {:<8}),",
                 ::kernel::sched::current_id(),
                 unsafe {::CPU_ID },
                 ::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst));

        dummy_work();
    }
    ::kernel::sched::create_task(task);
}

pub fn start() {
    ::kernel::sched::create_task(task);
    ::kernel::sched::create_user_task(
        ::kernel::user::get_user_program(), ::kernel::user::get_user_program_size(),
        0x60000
    );
}
