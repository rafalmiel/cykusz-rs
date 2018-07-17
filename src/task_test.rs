
const WORK_COUNT: usize = 10000;
const ITERS: usize = 1;

fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..WORK_COUNT {
        unsafe {
            let _ = a.read_volatile();
        }
    }

}

fn task_1() {
    let id = unsafe {::CPU_ID * 2 };
    for _ in 0..ITERS {
        print!("{},", id);

        dummy_work();
    }
}

fn task_2() {
    let id = unsafe {::CPU_ID * 2 + 1 };
    for _ in 0..ITERS {
        print!("{},", id);

        dummy_work();
    }
}

pub fn start() {
    //::kernel::sched::create_task(task_1);
    //::kernel::sched::create_task(task_2);
    //::bochs();
    ::kernel::sched::create_user_task(
        unsafe {::core::mem::transmute::<usize, fn() -> ()>(0x40000) },
        0x60000, 4096
    );
}
