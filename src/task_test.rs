
const WORK_COUNT: usize = 0x1000000;
const ITERS: usize = <usize>::max_value();

fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..WORK_COUNT {
        unsafe {
            let _ = a.read_volatile();
        }
    }

}

fn task() {
    let id = unsafe {::CPU_ID };
    for i in 0..ITERS {
        print!("K({} {:4}),", id, i);

        dummy_work();
    }
}

pub fn start() {
    ::kernel::sched::create_task(task);
    ::kernel::sched::create_user_task(
        unsafe {::core::mem::transmute::<usize, fn() -> ()>(0x40000) },
        0x60000, 4096
    );
}
