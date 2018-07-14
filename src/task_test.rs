fn task_1() {
//    let a = &3 as *const i32;

    let mut cnt = 0;

    let id = unsafe {::CPU_ID * 2 };
    loop {
        print!("{},", id);

        // Dummy work
//        for i in 1..10000 {
//            unsafe {
//                let _ = a.read_volatile();
//            }
//        }

        cnt += 1;

        if cnt == 10 {
            break;
        }

    }
}

fn task_2() {
//    let a = &3 as *const i32;

    let mut cnt = 0;

    let id = unsafe {::CPU_ID * 2 + 1 };
    loop {
        print!("{},", id);


        // Dummy work
//        for i in 1..10000 {
//            unsafe {
//                let _ = a.read_volatile();
//            }
//        }

        cnt += 1;

        if cnt == 10 {
            break;
        }

    }
}

pub fn start() {
    ::kernel::sched::create_task(task_1);
    ::kernel::sched::create_task(task_2);
}
