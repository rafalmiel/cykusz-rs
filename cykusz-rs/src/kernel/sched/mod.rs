use alloc::sync::Arc;
use core::any::Any;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use downcast_rs::DowncastSync;
use spin::Once;

use syscall_defs::exec::ExeArgs;
use syscall_defs::SyscallError;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::round_robin::RRScheduler;
use crate::kernel::sched::task_container::TaskContainer;
use crate::kernel::session::sessions;
use crate::kernel::signal::SignalResult;
use crate::kernel::task::{ArcTask, Task};

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

bitflags! {
    #[derive(Copy, Clone)]
    pub struct SleepFlags: u64 {
        const NON_INTERRUPTIBLE = 1;
    }
}

mod round_robin;
mod task_container;

static NEW_TASK_ID: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static LOCK_PROTECTION: AtomicBool = AtomicBool::new(false);

pub fn new_task_tid() -> usize {
    NEW_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

pub trait SchedulerInterface: Send + Sync + DowncastSync {
    fn init(&self) {}
    fn reschedule(&self) -> bool;
    fn current_task<'a>(&self) -> &'a Task;
    fn current_id(&self) -> isize {
        self.current_task().tid() as isize
    }
    fn queue_task(&self, task: ArcTask, alloc_cpu: bool);
    fn sleep(&self, until: Option<usize>, flags: SleepFlags) -> SignalResult<()>;
    fn wake(&self, task: ArcTask);
    fn wake_as_next(&self, task: ArcTask);
    fn cont(&self, task: ArcTask);
    fn stop(&self, sig: usize);
    fn exit(&self, status: syscall_defs::waitpid::Status) -> !;
    fn exit_thread(&self) -> !;
    fn debug(&self);
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

    pub fn current_task<'a>(&self) -> &'a Task {
        self.sched.current_task()
    }

    pub fn current_id(&self) -> isize {
        self.current_task().tid() as isize
    }

    pub fn internal(&self) -> &Arc<dyn SchedulerInterface> {
        &self.sched
    }

    pub fn reschedule(&self) -> bool {
        self.sched.reschedule()
    }

    pub fn register_task(&self, task: &ArcTask) {
        self.tasks.register_task(task.clone());
    }

    fn create_task(&self, fun: fn()) -> ArcTask {
        dbgln!(sched, "create task");
        let task = Task::new_kern(fun);
        dbgln!(sched, "task created");

        self.tasks.register_task(task.clone());

        dbgln!(sched, "task registered");

        sessions().register_process(task.clone());

        dbgln!(sched, "task sched registered");

        self.sched.queue_task(task.clone(), false);

        dbgln!(sched, "task queued");

        task
    }

    fn create_param_task(&self, fun: usize, val: usize) -> ArcTask {
        let task = Task::new_param_kern(fun, val);

        self.tasks.register_task(task.clone());

        sessions().register_process(task.clone());

        self.sched.queue_task(task.clone(), false);

        task
    }

    #[allow(unused)]
    pub fn as_impl<T: SchedulerInterface>(&self) -> &T {
        match self.sched.downcast_ref::<T>() {
            Some(e) => e,
            _ => panic!("invalid conversion"),
        }
    }

    fn sleep(&self, time_ns: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
        self.sched.sleep(time_ns, flags)
    }

    fn wake(&self, task: ArcTask) {
        self.sched.wake(task);
    }

    fn wake_as_next(&self, task: ArcTask) {
        self.sched.wake_as_next(task);
    }

    fn stop(&self, sig: usize) {
        let current = current_task_ref();

        if current.is_process_leader() {
            current.stop_threads();

            self.sched.stop(sig);
        } else {
            if let Some(parent) = current.get_parent() {
                parent.signal_thread(sig);
            }
        }
    }

    fn stop_thread(&self) {
        self.sched.stop(syscall_defs::signal::SIGSTOP);
    }

    fn cont(&self, task: ArcTask) {
        if task.is_process_leader() {
            dbgln!(task_stop, "cont threads!");
            self.sched.cont(task.clone());
        } else { match task.get_parent() { Some(parent) => {
            cont(parent);
        } _ => {}}}
    }

    fn debug(&self) {
        self.sched.debug()
    }

    fn cont_thread(&self, thread: ArcTask) {
        self.sched.cont(thread.clone());
    }

    fn close_all_tasks(&self) {
        self.tasks.close_all_tasks();
    }

    pub fn fork(&self) -> ArcTask {
        let current = self.sched.current_task();

        let forked = current.fork();

        dbgln!(task, "fork {} -> {}", current.tid(), forked.tid());

        self.tasks.register_task(forked.clone());

        sessions().register_process(forked.clone());

        self.sched.queue_task(forked.clone(), true);

        forked
    }

    pub fn exec(
        &self,
        exe: DirEntryItem,
        args: Option<ExeArgs>,
        envs: Option<ExeArgs>,
    ) -> Result<!, SyscallError> {
        let current = self.sched.current_task();

        dbgln!(task, "exec {} -> {}", current.tid(), exe.full_path());

        if current.is_process_leader() {
            current.terminate_threads();

            current.exec(exe, args, envs)
        } else {
            if current
                .process_leader()
                .signals()
                .setup_sig_exec(sigexec_exec, Arc::new(ExecParams { exe, args, envs }))
            {
                current.process_leader().wake_up_as_next();
            }

            self.exit_thread();
        }
    }

    pub fn spawn_thread(&self, entry: VirtAddr, stack: VirtAddr) -> ArcTask {
        let current = self.sched.current_task();

        let thread = current.spawn_thread(entry, stack);

        dbgln!(task, "{} spawn thread {}", current.tid(), thread.tid());

        self.tasks.register_task(thread.clone());

        self.sched.queue_task(thread.clone(), true);

        thread
    }

    pub fn exit(&self, status: syscall_defs::waitpid::Status) -> ! {
        let current = current_task_ref();

        dbgln!(
            task,
            "exit tid {} is pl: {}, sc: {}, wc: {} status {:?}",
            current.tid(),
            current.is_process_leader(),
            ArcTask::strong_count(&current.me()),
            ArcTask::weak_count(&current.me()),
            status
        );

        assert_eq!(current.locks(), 0, "Killing thread holding a locks");

        if current.is_process_leader() {
            current.terminate_threads();

            current.close_all_files();

            self.tasks.remove_task(current.tid());

            current.migrate_children_to_init();

            //dbgln!(mem, "free mem: {} {}", crate::kernel::mm::heap::heap_mem(), crate::arch::mm::phys::free_mem());

            self.sched.exit(status)
        } else {
            if !status.is_signaled() {
                current
                    .process_leader()
                    .signal_thread(syscall_defs::signal::SIGKILL);
            } else {
                current
                    .process_leader()
                    .signal_thread(status.which_signal() as usize);
                self.cont(current.process_leader());
            }

            self.exit_thread();
        }
    }

    pub fn exit_thread(&self) -> ! {
        let task = current_task_ref();

        if task.is_process_leader() {
            logln_disabled!("[ WARN ] exit thread of a process leader");

            self.exit(syscall_defs::waitpid::Status::Exited(0));
        } else {
            self.tasks.remove_task(task.tid());

            self.sched.exit_thread();
        }
    }

    pub fn get_task(&self, tid: usize) -> Option<ArcTask> {
        self.tasks.get(tid)
    }

    pub fn queue(&self, task: ArcTask, alloc_cpu: bool) {
        self.sched.queue_task(task, alloc_cpu);
    }
}

pub struct ExecParams {
    exe: DirEntryItem,
    args: Option<ExeArgs>,
    envs: Option<ExeArgs>,
}

fn sigexec_exec(param: Arc<dyn Any + Send + Sync>) {
    if let Ok(param) = param.downcast::<ExecParams>() {
        let exe = param.exe.clone();
        let args = param.args.clone();
        let envs = param.envs.clone();

        drop(param);

        exec(exe, args, envs).unwrap();
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
    if let Some(sched) = SCHEDULER.get() {
        sched
    } else {
        panic!("sched is None!");
    }
}

pub(in crate::kernel::sched) fn finalize() {
    crate::kernel::int::finish();
    crate::kernel::timer::reset_counter();
}

pub(in crate::kernel::sched) fn register_task(task: &ArcTask) {
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

pub fn internal() -> Arc<dyn SchedulerInterface> {
    scheduler().internal().clone()
}

pub fn get_task(tid: usize) -> Option<ArcTask> {
    scheduler().get_task(tid)
}

pub fn current_task() -> ArcTask {
    scheduler().current_task().me()
}

pub fn current_task_ref<'a>() -> &'a Task {
    scheduler().current_task()
}

pub fn current_id() -> isize {
    scheduler().current_id()
}

pub fn exit(status: syscall_defs::waitpid::Status) -> ! {
    scheduler().exit(status)
}

pub fn exit_thread() -> ! {
    scheduler().exit_thread();
}

pub fn create_task(fun: fn()) -> ArcTask {
    scheduler().create_task(fun)
}

pub fn create_param_task(fun: usize, val: usize) -> ArcTask {
    scheduler().create_param_task(fun, val)
}

pub fn queue_task(task: ArcTask, alloc_cpu: bool) {
    scheduler().queue(task, alloc_cpu);
}

pub fn sleep(time_ns: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
    scheduler().sleep(time_ns, flags)
}

pub fn wake(task: ArcTask) {
    scheduler().wake(task);
}

pub fn wake_as_next(task: ArcTask) {
    scheduler().wake_as_next(task);
}

pub fn stop(sig: usize) {
    scheduler().stop(sig);
}

pub fn stop_thread() {
    scheduler().stop_thread();
}

pub fn cont(task: ArcTask) {
    scheduler().cont(task);
}

pub fn debug() {
    scheduler().debug()
}

pub fn cont_thread(thread: ArcTask) {
    scheduler().cont_thread(thread);
}

pub fn fork() -> ArcTask {
    scheduler().fork()
}

pub fn exec(
    exe: DirEntryItem,
    args: Option<ExeArgs>,
    envs: Option<ExeArgs>,
) -> Result<!, SyscallError> {
    scheduler().exec(exe, args, envs)
}

pub fn spawn_thread(entry: VirtAddr, stack: VirtAddr) -> ArcTask {
    scheduler().spawn_thread(entry, stack)
}

pub fn close_all_tasks() {
    scheduler().close_all_tasks();
}

fn lock_protection_ready() -> bool {
    crate::kernel::tls::is_ready() && LOCK_PROTECTION.load(Ordering::SeqCst)
}

pub fn preempt_disable() -> bool {
    if lock_protection_ready() {
        let current = scheduler().current_task();

        current.inc_locks();

        return true;
    }

    false
}

pub fn current_locks_var<'a>() -> Option<&'a AtomicUsize> {
    if lock_protection_ready() {
        Some(current_task_ref().locks_ref())
    } else {
        None
    }
}

pub fn preempt_enable() {
    if lock_protection_ready() {
        let current = scheduler().current_task();

        if current.dec_locks() == 0 && current.to_reschedule() {
            reschedule();
        }
    }
}

pub fn enable_lock_protection() {
    LOCK_PROTECTION.store(true, Ordering::SeqCst);
}
