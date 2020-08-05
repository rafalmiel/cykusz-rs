use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::kernel::sched::current_task;
use crate::kernel::sync::{IrqGuard, RwSpin};
use crate::kernel::task::Task;

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

pub trait TimerObject: Sync + Send {
    fn call(&self);
}

pub struct Timer {
    obj: Arc<dyn TimerObject>,
    task: Weak<Task>,
    id: usize,
    timeout: AtomicUsize, //in ms
    terminate: AtomicBool,
}

impl Timer {
    pub fn halt(&self) {
        if let Some(t) = &self.task.upgrade() {
            t.set_halted(true);
        }
    }

    pub fn resume(&self) {
        if let Some(t) = &self.task.upgrade() {
            t.set_halted(false);
        }
    }

    pub fn is_terminating(&self) -> bool {
        self.terminate.load(Ordering::SeqCst)
    }

    pub fn set_terminate(&self) {
        self.terminate.store(true, Ordering::SeqCst);
        if let Some(t) = &self.task.upgrade() {
            t.wake_up();
        }
    }

    pub fn timeout(&self) -> usize {
        self.timeout.load(Ordering::SeqCst)
    }

    pub fn set_timeout(&self, val: usize) {
        self.timeout.store(val, Ordering::SeqCst);
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.set_terminate();
    }
}

static TIMERS: RwSpin<BTreeMap<usize, Arc<Timer>>> = RwSpin::new(BTreeMap::new());

fn timer_fun(id: usize) {
    loop {
        let task = current_task();

        let timer = {
            if let Some(timer) = TIMERS.read().get(&id) {
                timer.clone()
            } else {
                break;
            }
        };

        task.sleep(timer.timeout() * 1_000_000);

        if timer.is_terminating() {
            break;
        } else {
            timer.obj.call()
        }
    }

    TIMERS.write().remove(&id);
}

pub fn create_timer(timer: Arc<dyn TimerObject>, timeout: usize) -> Arc<Timer> {
    let id = TIMER_ID.fetch_add(1, Ordering::SeqCst);

    let mut timers = TIMERS.write();

    let task = {
        let _irq = IrqGuard::new();
        let task = crate::kernel::sched::create_param_task(timer_fun as usize, id);
        task.set_halted(true);
        task
    };

    let t = Arc::new(Timer {
        obj: timer,
        task: Arc::downgrade(&task),
        id,
        timeout: AtomicUsize::new(timeout),
        terminate: AtomicBool::new(false),
    });

    timers.insert(id, t.clone());

    t
}

pub fn setup() {
    crate::arch::timer::setup(timer_handler);
}

pub fn start() {
    crate::arch::timer::start();
}

pub fn reset_counter() {
    crate::arch::timer::reset_counter();
}

fn timer_handler() {
    //println!("Timer handler");
    crate::kernel::sched::reschedule();
}

pub fn early_sleep(ms: u64) {
    crate::arch::timer::early_sleep(ms);
}

pub fn busy_sleep(ns: u64) {
    crate::arch::timer::busy_sleep(ns)
}

pub fn current_ns() -> u64 {
    crate::arch::timer::current_ns()
}
