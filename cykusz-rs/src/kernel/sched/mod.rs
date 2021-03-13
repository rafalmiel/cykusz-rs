use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use downcast_rs::DowncastSync;
use spin::Once;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::sched::round_robin::RRScheduler;
use crate::kernel::sched::task_container::TaskContainer;
use crate::kernel::signal::SignalResult;
use crate::kernel::task::Task;

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

mod round_robin;
mod task_container;

static NEW_TASK_ID: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static LOCK_PROTECTION: AtomicBool = AtomicBool::new(false);

pub fn new_task_id() -> usize {
    NEW_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

pub trait SchedulerInterface: Send + Sync + DowncastSync {
    fn init(&self) {}
    fn reschedule(&self) -> bool;
    fn current_task(&self) -> Arc<Task>;
    fn current_id(&self) -> isize {
        self.current_task().id() as isize
    }
    fn queue_task(&self, task: Arc<Task>);
    fn sleep(&self, until: Option<usize>) -> SignalResult<()>;
    fn wake(&self, task: Arc<Task>);
    fn exit(&self, status: isize) -> !;
}

impl_downcast!(sync SchedulerInterface);

struct Scheduler {
    sched: Arc<dyn SchedulerInterface>,

    tasks: TaskContainer,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            sched: RRScheduler::new(),

            tasks: TaskContainer::default(),
        }
    }

    fn init(&self) {
        self.sched.init();
    }

    pub fn current_task(&self) -> Arc<Task> {
        self.sched.current_task()
    }

    pub fn current_id(&self) -> isize {
        self.current_task().id() as isize
    }

    pub fn reschedule(&self) -> bool {
        self.sched.reschedule()
    }

    pub fn register_task(&self, task: &Arc<Task>) {
        self.tasks.register_task(task.clone());
    }

    fn create_task(&self, fun: fn()) -> Arc<Task> {
        let task = Task::new_kern(fun);

        self.tasks.register_task(task.clone());

        self.sched.queue_task(task.clone());

        task
    }

    fn create_param_task(&self, fun: usize, val: usize) -> Arc<Task> {
        let task = Task::new_param_kern(fun, val);

        self.tasks.register_task(task.clone());

        self.sched.queue_task(task.clone());

        task
    }

    fn create_user_task(&self, exe: DirEntryItem) -> Arc<Task> {
        let task = Task::new_user(exe);

        self.tasks.register_task(task.clone());

        self.sched.queue_task(task.clone());

        task
    }

    pub fn as_impl<T: SchedulerInterface>(&self) -> &T {
        match self.sched.downcast_ref::<T>() {
            Some(e) => e,
            _ => panic!("invalid conversion"),
        }
    }

    fn sleep(&self, time_ns: Option<usize>) -> SignalResult<()> {
        self.sched.sleep(time_ns)
    }

    fn wake(&self, task: Arc<Task>) {
        self.sched.wake(task);
    }

    fn close_all_tasks(&self) {
        self.tasks.close_all_tasks();
    }

    pub fn fork(&self) -> Arc<Task> {
        let current = self.sched.current_task();

        let forked = current.fork();

        self.tasks.register_task(forked.clone());

        self.sched.queue_task(forked.clone());

        forked
    }

    pub fn exec(&self, exe: DirEntryItem) -> ! {
        let current = self.sched.current_task();

        let execd = current.exec(exe);

        self.tasks.register_task(execd.clone());

        self.sched.queue_task(execd);

        drop(current);

        self.sched.exit(0);
    }

    pub fn exit(&self) -> ! {
        let current = current_task();

        self.tasks.remove_task(current.id());

        current.migrate_children_to_parent();

        drop(current);

        self.sched.exit(0)
    }
}

static SCHEDULER: Once<Scheduler> = Once::new();

pub fn init() {
    SCHEDULER.call_once(|| Scheduler::new());

    scheduler().init();

    enable_lock_protection();
}

pub fn init_ap() {
    scheduler().init();

    enable_lock_protection();
}

fn scheduler() -> &'static Scheduler {
    SCHEDULER.get().unwrap()
}

pub(in crate::kernel::sched) fn finalize() {
    crate::kernel::int::finish();
    crate::kernel::timer::reset_counter();
}

pub(in crate::kernel::sched) fn register_task(task: &Arc<Task>) {
    scheduler().register_task(task);
}

pub fn reschedule() -> bool {
    let scheduler = scheduler();

    {
        let current = scheduler.current_task();

        if current.locks() > 0 {
            current.set_to_reschedule(true);
            finalize();
            return false;
        }
    }

    scheduler.reschedule()
}

pub fn current_task() -> Arc<Task> {
    scheduler().current_task()
}

pub fn current_id() -> isize {
    scheduler().current_id()
}

pub fn task_finished() -> ! {
    scheduler().exit()
}

pub fn create_task(fun: fn()) -> Arc<Task> {
    scheduler().create_task(fun)
}

pub fn create_param_task(fun: usize, val: usize) -> Arc<Task> {
    scheduler().create_param_task(fun, val)
}

pub fn create_user_task(exe: DirEntryItem) -> Arc<Task> {
    scheduler().create_user_task(exe)
}

pub fn sleep(time_ns: Option<usize>) -> SignalResult<()> {
    scheduler().sleep(time_ns)
}

pub fn wake(task: Arc<Task>) {
    scheduler().wake(task);
}

pub fn fork() -> Arc<Task> {
    scheduler().fork()
}

pub fn exec(exe: DirEntryItem) -> ! {
    scheduler().exec(exe)
}

pub fn close_all_tasks() {
    scheduler().close_all_tasks();
}

fn lock_protection_ready() -> bool {
    crate::kernel::tls::is_ready() && LOCK_PROTECTION.load(Ordering::SeqCst)
}

pub fn enter_critical_section() -> bool {
    if lock_protection_ready() {
        let current = scheduler().current_task();

        current.inc_locks();

        return true;
    }

    return false;
}

pub fn leave_critical_section() {
    if lock_protection_ready() {
        let current = scheduler().current_task();

        current.dec_locks();
    }
}

pub fn enable_lock_protection() {
    LOCK_PROTECTION.store(true, Ordering::SeqCst);
}
