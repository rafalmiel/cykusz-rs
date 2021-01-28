#![allow(dead_code, unused_imports)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::AtomicU64;

use intrusive_collections::LinkedListLink;

use crate::arch::raw::mm::MappedAddr;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_real_path, root_dentry, LookupMode};
use crate::kernel::sched::current_task;
use crate::kernel::syscall::sys::sys_sleep;
use crate::kernel::timer::{create_timer, Timer, TimerObject};
use syscall_defs::OpenFlags;

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

//struct TimerTest {}
//
//impl TimerObject for TimerTest {
//    fn call(&self) {
//        println!("Timer called");
//    }
//}
//
//static mut TIMER: Option<Arc<Timer>> = None;
//
//struct Element {
//    link: LinkedListLink,
//    val: u8,
//}
//
//impl Element {
//    pub fn new(val: u8) -> Arc<Element> {
//        Arc::new(Element {
//            link: LinkedListLink::new(),
//            val,
//        })
//    }
//}
//
//intrusive_adapter!(ElementAdapter = Arc<Element>: Element {link: LinkedListLink});

fn load_bin(path: &str) -> Option<Vec<u8>> {
    let task = current_task();
    task.set_cwd(root_dentry().unwrap().clone());

    let shell = lookup_by_real_path(Path::new(path), LookupMode::None).expect("Shell not found");

    if let Some(fd) = task.open_file(shell, OpenFlags::RDONLY) {
        if let Some(handle) = task.get_handle(fd) {
            let mut code = Vec::<u8>::new();
            code.resize(1024, 0);
            let mut size = 0;

            while let Ok(read) = handle.read(&mut code.as_mut_slice()[size..size + 1024]) {
                size += read;

                if read < 1024 {
                    code.resize(size, 0);
                    break;
                }

                code.resize(size + 1024, 0);
            }

            task.close_file(fd);

            return Some(code);
        }

        task.close_file(fd);
    }

    None
}

pub fn start() {
    if let Some(code) = load_bin("/bin/shell") {
        println!("Exec shell...");
        crate::kernel::sched::create_user_task(
            MappedAddr(code.as_ptr() as usize),
            code.len() as u64,
        );
    } else {
        println!("Failed to exec shell");
    }

    //crate::kernel::sched::create_task(task2);
    //crate::kernel::sched::create_task(task);
    //crate::kernel::sched::create_task(task);
    //crate::kernel::sched::create_user_task(
    //    crate::kernel::user::get_user_program(),
    //    crate::kernel::user::get_user_program_size(),
    //);

    //if cfg!(disabled) {
    //    unsafe {
    //        if let Some(t) = &TIMER {
    //            t.terminate();
    //            TIMER = None;
    //        } else {
    //            let timer = create_timer(Arc::new(TimerTest {}), 1000);
    //            TIMER = Some(timer);
    //            TIMER.as_ref().unwrap().resume();
    //        }
    //    }
    //}
}
