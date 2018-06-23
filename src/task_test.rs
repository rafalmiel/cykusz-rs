use arch::task::Task;

static mut TASK1: Task = Task::empty();
static mut TASK2: Task = Task::empty();

pub fn task_1() {
    loop {
        unsafe {
            switch!(TASK1, TASK2);
        }
    }
}

pub fn task_2() {
    loop {
        unsafe {
            switch!(TASK2, TASK1);
        }
    }
}

pub fn start() {
    unsafe {
        TASK1 = Task::create_kernel_task(task_1);
        TASK2 = Task::create_kernel_task(task_2);
    }

    let mut t0 = Task::empty();

    unsafe {
        switch!(t0, TASK1);
    }
}
