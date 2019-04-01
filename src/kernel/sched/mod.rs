use core::sync::atomic::{AtomicBool, AtomicUsize};
use core::sync::atomic::Ordering;

use spin::Once;

use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::IrqGuard;

use self::cpu_queue::CpuQueue;
use self::cpu_queues::CpuQueues;
use self::task_container::TaskContainer;

#[macro_export]
macro_rules! switch {
    ($ctx1: expr, $ctx2: expr) => (
        $crate::arch::task::switch(&mut $ctx1.arch_task_mut(), &$ctx2.arch_task());
    )
}
#[macro_export]
macro_rules! activate_task {
    ($ctx1: expr) => (
        $crate::arch::task::activate_task(&$ctx1.arch_task());
    )
}
mod task_container;
mod cpu_queues;
mod cpu_queue;

static NEW_TASK_ID: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static LOCK_PROTECTION: AtomicBool = AtomicBool::new(false);

#[thread_local]
static LOCK_PROTECTION_ENTERED: AtomicBool = AtomicBool::new(false);

#[thread_local]
static CURRENT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

pub fn new_task_id() -> usize {
    NEW_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn current_id() -> usize {
    CURRENT_TASK_ID.load(Ordering::SeqCst)
}

#[derive(Default)]
struct Scheduler {
    tasks: TaskContainer,
    cpu_queues: CpuQueues,
}

impl Scheduler {

    fn add_task(&self, fun: fn()) {
        let _g = IrqGuard::new();

        let task = self.tasks.add_task(fun);

        self.cpu_queues.add_task(task);
    }

    fn add_user_task(&self, fun: MappedAddr, code_size: usize, stack: usize) {
        let _g = IrqGuard::new();

        let task = self.tasks.add_user_task(fun, code_size, stack);

        self.cpu_queues.add_task(task);
    }

    fn schedule_next(&self) {

        self.cpu_queues.schedule_next();
    }

    fn reschedule(&self) -> bool {
        let _g = IrqGuard::new();

        self.cpu_queues.reschedule()
    }

    fn enter_critical_section(&self) {
        let _g = IrqGuard::new();

        self.cpu_queues.enter_critical_section();
    }

    fn leave_critical_section(&self) {
        let _g = IrqGuard::new();

        self.cpu_queues.leave_critical_section();
    }

    fn current_task_finished(&self) {
        let _g = IrqGuard::new();

        self.tasks.remove_task(CURRENT_TASK_ID.load(Ordering::SeqCst));
        self.cpu_queues.current_task_finished();
    }
}

static SCHEDULER: Once<Scheduler> = Once::new();

fn scheduler() -> &'static Scheduler {
    SCHEDULER.r#try().expect("Scheduler not initialized")
}

fn scheduler_main() {
    loop {
        scheduler().schedule_next();
    }
}

pub fn reschedule() -> bool {
    scheduler().reschedule()
}

pub fn task_finished() {
    scheduler().current_task_finished();
}

pub fn create_task(fun: fn()) {
    scheduler().add_task(fun);
}

pub fn create_user_task(fun: MappedAddr, code_size: u64, stack: usize) {
    scheduler().add_user_task(fun, code_size as usize, stack);
}

fn lock_protection_ready() -> bool {
    crate::kernel::tls::is_ready()
        && LOCK_PROTECTION.load(Ordering::SeqCst)
        && !LOCK_PROTECTION_ENTERED.load(Ordering::SeqCst)
}

pub fn enter_critical_section() -> bool {
    if lock_protection_ready() {

        scheduler().enter_critical_section();

        return true;
    }

    return false;
}

pub fn leave_critical_section() {
    if lock_protection_ready() {

        scheduler().leave_critical_section();
    }
}

pub fn enable_lock_protection() {
    LOCK_PROTECTION.store(true, Ordering::SeqCst);
}

pub fn init() {
    SCHEDULER.call_once(|| {
        Scheduler::default()
    });
}

