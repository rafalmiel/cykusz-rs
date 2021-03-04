#![allow(dead_code, unused_imports)]

use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::AtomicU64;

use intrusive_collections::LinkedListLink;

use syscall_defs::OpenFlags;

use crate::arch::raw::mm::MappedAddr;
use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_real_path, root_dentry, LookupMode};
use crate::kernel::mm::heap::{leak_catcher, HeapDebug};
use crate::kernel::sched::{create_user_task, current_task};
use crate::kernel::syscall::sys::sys_sleep;
use crate::kernel::task::filetable::FileHandle;
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

    let fh = FileHandle::new(0, shell, OpenFlags::RDONLY)?;

    if let Ok(r) = fh.read_all() {
        Some(r)
    } else {
        None
    }
}

pub fn start() {
    //if let Some(code) = load_bin("/bin/shell") {
    //    println!("Exec shell...");
    //    crate::kernel::sched::create_user_task(code.as_slice());
    //} else {
    //    println!("Failed to exec shell");
    //}
    let task = current_task();
    task.set_cwd(root_dentry().unwrap().clone());

    let shell =
        lookup_by_real_path(Path::new("/bin/shell"), LookupMode::None).expect("Shell not found");

    let task = create_user_task(shell);

    task.open_file(lookup_by_real_path(Path::new("/dev/stdin"), LookupMode::None).expect("stdin open failed"), OpenFlags::RDONLY);

    let stdout = lookup_by_real_path(Path::new("/dev/stdout"), LookupMode::None).expect("stdout open failed");

    task.open_file(stdout.clone(), OpenFlags::WRONLY);
    task.open_file(stdout, OpenFlags::WRONLY);

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
