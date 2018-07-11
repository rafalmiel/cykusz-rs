use crate::arch::task::Task;

static mut TASK1: Task = Task::empty();
static mut TASK2: Task = Task::empty();

pub fn task_1() {
    unsafe {
        switch!(TASK1, TASK2);
        switch!(TASK1, TASK2);
        switch!(TASK1, TASK2);
        switch!(TASK1, TASK2);
        switch!(TASK1, TASK2);
    }
    println!("FINISHED 1");
    loop{}
}

pub fn task_2() {
    unsafe {
        switch!(TASK2, TASK1);
        switch!(TASK2, TASK1);
        switch!(TASK2, TASK1);
        switch!(TASK2, TASK1);
    }
    println!("FINISHED 2");
    unsafe {
        switch!(TASK2, TASK1);
    }
    loop{}
}

pub fn start() {
    unsafe {
        TASK1 = Task::create_kernel_task(task_1);
        TASK2 = Task::create_kernel_task(task_2);
    }

    let mut t0 = Task::empty();

    unsafe {
        asm!("xchg %bx, %bx");
        switch!(t0, TASK1);
    }
}
