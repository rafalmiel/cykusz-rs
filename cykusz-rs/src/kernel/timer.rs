use alloc::sync::{Arc, Weak};
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use intrusive_collections::{LinkedList, LinkedListLink};

use crate::kernel::sched::current_task;
use crate::kernel::sync::{Spin, SpinGuard};
use crate::kernel::utils::wait_queue::WaitQueue;

pub trait TimerObject: Send + Sync {
    fn call(&self);
}

pub struct Timer {
    timeout: AtomicU64,
    obj: Arc<dyn TimerObject>,
    self_ref: Weak<Timer>,
    link: LinkedListLink,
}

impl Drop for Timer {
    fn drop(&mut self) {
        println!("[ TCP ] Timer dropped");
    }
}

unsafe impl Sync for Timer {}

unsafe impl Send for Timer {}

impl Timer {
    fn new(obj: Arc<dyn TimerObject>) -> Arc<Timer> {
        Arc::new_cyclic(|me| Timer {
            timeout: AtomicU64::new(0),
            obj,
            self_ref: me.clone(),
            link: LinkedListLink::new(),
        })
    }

    fn call(&self) {
        self.obj.call();
    }

    fn unlink_locked(&self, timers: &mut SpinGuard<LinkedList<TimerAdapter>>) {
        if self.link.is_linked() {
            let mut c = unsafe { timers.cursor_mut_from_ptr(self as *const Timer) };

            c.remove();
        }
    }

    fn link_locked(&self, timers: &mut SpinGuard<LinkedList<TimerAdapter>>, timeout: u64) {
        if let Some(timer) = self.self_ref.upgrade() {
            timer.set_timeout(timeout);

            if let Some(ptr) = timers.iter().find_map(|e| {
                if e.timeout() > timer.timeout() {
                    Some(e as *const Timer)
                } else {
                    None
                }
            }) {
                let mut c = unsafe { timers.cursor_mut_from_ptr(ptr as *const _) };

                c.insert_before(timer);
            } else {
                timers.push_back(timer);
            }

            TIMERS_WQ.notify_one();
        }
    }

    pub fn disable(&self) {
        let mut timers = TIMERS.lock();

        self.unlink_locked(&mut timers);
    }

    fn timeout(&self) -> u64 {
        self.timeout.load(Ordering::SeqCst)
    }

    fn set_timeout(&self, val: u64) {
        self.timeout
            .store(current_ns() + val * 1_000_000, Ordering::SeqCst);
    }

    pub fn enabled(&self) -> bool {
        self.link.is_linked()
    }

    pub fn start_with_timeout(&self, timeout: u64) {
        let mut timers = TIMERS.lock();

        self.unlink_locked(&mut timers);
        self.link_locked(&mut timers, timeout);
    }
}

intrusive_adapter!(TimerAdapter = Arc<Timer>: Timer {link: LinkedListLink});

lazy_static! {
    static ref TIMERS: Spin<LinkedList<TimerAdapter>> =
        Spin::new(LinkedList::new(TimerAdapter::new()));
}

static TIMERS_WQ: WaitQueue = WaitQueue::new();

fn check_timers() {
    let time = current_ns();

    let mut timers = TIMERS_WQ.wait_lock_for(&TIMERS, |lck| !lck.is_empty());

    loop {
        if let Some(timer) = timers.front().get() {
            if timer.timeout() <= time {
                let t = timer.self_ref.upgrade().unwrap();

                timers.pop_front();

                drop(timers);

                t.call();

                timers = TIMERS.lock();
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn timer_fun() {
    let task = current_task();
    loop {
        check_timers();

        // check timers every 100 ms
        task.sleep(100_000_000);
    }
}

pub struct TimerCallback<T: Send + Sync> {
    obj: Weak<T>,
    fun: fn(&T),
}

impl<T: Send + Sync> TimerObject for TimerCallback<T> {
    fn call(&self) {
        if let Some(s) = self.obj.upgrade() {
            (self.fun)(&s)
        }
    }
}

impl<T: Send + Sync> TimerCallback<T> {
    pub fn new(sock: Weak<T>, cb: fn(&T)) -> Arc<TimerCallback<T>> {
        Arc::new(TimerCallback { obj: sock, fun: cb })
    }
}

pub fn create_timer(obj: Arc<dyn TimerObject>) -> Arc<Timer> {
    let timer = Timer::new(obj);

    return timer;
}

pub fn setup() {
    crate::kernel::sched::create_task(timer_fun);

    crate::arch::timer::setup(timer_handler);
}

pub fn start() {
    crate::arch::timer::start();
}

pub fn reset_counter() {
    crate::arch::timer::reset_counter();
}

fn timer_handler() {
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
