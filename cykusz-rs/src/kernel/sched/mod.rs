use alloc::sync::Arc;

use core::sync::atomic::Ordering;
use core::sync::atomic::{AtomicBool, AtomicUsize};

use spin::Once;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::sync::IrqGuard;
use crate::kernel::task::Task;

use self::cpu_queue::CpuQueue;
use self::cpu_queues::CpuQueues;
use self::task_container::TaskContainer;

#[macro_export]
macro_rules! switch {
    ($ctx1: expr, $ctx2: expr) => {
        $crate::arch::task::switch(&mut $ctx1.arch_task_mut(), &$ctx2.arch_task());
    };
}
#[macro_export]
macro_rules! activate_task {
    ($ctx1: expr) => {
        $crate::arch::task::activate_task(&$ctx1.arch_task());
    };
}
mod cpu_queue;
mod cpu_queues;
mod task_container;

static NEW_TASK_ID: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static LOCK_PROTECTION: AtomicBool = AtomicBool::new(false);

#[thread_local]
static LOCK_PROTECTION_ENTERED: AtomicBool = AtomicBool::new(false);

#[thread_local]
static CURRENT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

#[thread_local]
static QUEUE_LEN: AtomicUsize = AtomicUsize::new(0);

pub fn new_task_id() -> usize {
    NEW_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn current_id() -> usize {
    CURRENT_TASK_ID.load(Ordering::SeqCst)
}

pub fn queue_len() -> usize {
    QUEUE_LEN.load(Ordering::SeqCst)
}

#[derive(Default)]
struct Scheduler {
    tasks: TaskContainer,
    cpu_queues: CpuQueues,
}

impl Scheduler {
    fn add_task(&self, fun: fn()) -> Arc<Task> {
        let _g = IrqGuard::new();

        let task = self.tasks.add_task(fun);

        self.cpu_queues.add_task(task.clone());

        task
    }

    fn add_param_task(&self, fun: usize, val: usize) -> Arc<Task> {
        let _g = IrqGuard::new();

        let task = self.tasks.add_param_task(fun, val);

        self.cpu_queues.add_task(task.clone());

        task
    }

    fn add_user_task(&self, exe: DirEntryItem) -> Arc<Task> {
        let task = self.tasks.add_user_task(exe);

        let _g = IrqGuard::new();
        self.cpu_queues.add_task(task.clone());

        task
    }

    fn schedule_next(&self) {
        self.cpu_queues.schedule_next();
    }

    fn reschedule(&self) -> bool {
        let _g = IrqGuard::new();

        self.cpu_queues.reschedule()
    }

    fn activate_sched(&self) {
        let _g = IrqGuard::new();

        self.cpu_queues.activate_sched();
    }

    fn enter_critical_section(&self) {
        let _g = IrqGuard::new();

        self.cpu_queues.enter_critical_section();
    }

    fn leave_critical_section(&self) {
        let _g = IrqGuard::new();

        self.cpu_queues.leave_critical_section();
    }

    fn current_task_finished(&self) -> ! {
        current_task().vm().clear();
        let _g = IrqGuard::new();

        self.tasks.remove_task(current_id());
        self.cpu_queues.current_task_finished()
    }

    fn execd_task_finished(&self) -> ! {
        current_task().vm().clear();
        let _g = IrqGuard::new();

        self.cpu_queues.current_task_finished()
    }

    fn current_task(&self) -> Arc<Task> {
        let _g = IrqGuard::new();

        self.cpu_queues.current_task()
    }

    fn register_task(&self, task: Arc<Task>) {
        self.tasks.register_task(task)
    }

    fn close_all_tasks(&self) {
        self.tasks.close_all_tasks();
    }

    fn fork(&self) -> Arc<Task> {
        let task = self.tasks.fork();

        let _g = IrqGuard::new();
        self.cpu_queues.add_task(task.clone());

        task
    }

    fn exec(&self, exe: DirEntryItem) {
        let task = self.tasks.exec(exe);

        let _g = IrqGuard::new();
        self.cpu_queues.add_task(task);
    }

    fn init_tasks(&self) {
        self.cpu_queues.init_tasks();
    }
}

static SCHEDULER: Once<Scheduler> = Once::new();

fn scheduler() -> &'static Scheduler {
    SCHEDULER.get().expect("Scheduler not initialized")
}

fn scheduler_main() {
    loop {
        scheduler().schedule_next();
    }
}

pub(in crate::kernel::sched) fn register_task(task: Arc<Task>) {
    scheduler().register_task(task)
}

pub fn reschedule() -> bool {
    scheduler().reschedule()
}

pub fn activate_sched() {
    scheduler().activate_sched();
}

pub fn task_finished() -> ! {
    scheduler().current_task_finished()
}

pub fn create_task(fun: fn()) -> Arc<Task> {
    scheduler().add_task(fun)
}

pub fn create_param_task(fun: usize, val: usize) -> Arc<Task> {
    scheduler().add_param_task(fun, val)
}

pub fn create_user_task(exe: DirEntryItem) -> Arc<Task> {
    scheduler().add_user_task(exe)
}

pub fn fork() -> Arc<Task> {
    let new = scheduler().fork();

    new
}

pub fn exec(exe: DirEntryItem) -> ! {
    scheduler().exec(exe);

    scheduler().execd_task_finished();
}

pub fn current_task() -> Arc<Task> {
    scheduler().current_task()
}

pub fn close_all_tasks() {
    scheduler().close_all_tasks();
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
    SCHEDULER.call_once(|| Scheduler::default());

    scheduler().init_tasks();

    enable_lock_protection();
}
