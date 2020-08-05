#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::AtomicU64;

use crate::kernel::syscall::sys::sys_sleep;
use crate::kernel::timer::{create_timer, Timer, TimerObject};

//use core::sync::atomic::Ordering;

const WORK_COUNT: usize = 0x5000000;
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

#[thread_local]
static COUNT: AtomicU64 = AtomicU64::new(0);

fn task() {
    loop {
        crate::drivers::net::e1000::test::test();

        sys_sleep(2_000_000_000).expect("Sleep failed");
    }

    //COUNT.fetch_add(1, Ordering::SeqCst);

    //for _ in 0..ITERS {
    //    println!(
    //        "K( {:<6} PID: {:<6} CPU: {:<6} MEM: {:<8} LEN: {:<6}),",
    //        COUNT.load(Ordering::SeqCst),
    //        crate::kernel::sched::current_id(),
    //        unsafe { crate::CPU_ID },
    //        crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst),
    //        crate::kernel::sched::queue_len()
    //    );
    //}
    //crate::kernel::sched::create_task(task);
}

struct TimerTest {}

impl TimerObject for TimerTest {
    fn call(&self) {
        println!("Timer called");
    }
}

static mut TIMER: Option<Arc<Timer>> = None;

pub fn start() {
    //crate::kernel::sched::create_task(task2);
    //crate::kernel::sched::create_task(task);
    //crate::kernel::sched::create_task(task);
    crate::kernel::sched::create_user_task(
        crate::kernel::user::get_user_program(),
        crate::kernel::user::get_user_program_size(),
    );

    if cfg!(disabled) {
        unsafe {
            if let Some(t) = &TIMER {
                t.set_terminate();
                TIMER = None;
            } else {
                let timer = create_timer(Arc::new(TimerTest {}), 1000);
                TIMER = Some(timer);
                TIMER.as_ref().unwrap().resume();
            }
        }
    }
    //crate::kernel::sched::create_param_task(task as usize, 42);
}
