use crate::arch::ipi::IpiKind;
use crate::kernel;
use crate::kernel::sync::{IrqGuard, LockApi, Spin};
use crate::kernel::task::ArcTask;
use crate::kernel::utils::PerCpu;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use intrusive_collections::{LinkedList, LinkedListLink};
use spin::Once;

pub enum IpiTarget {
    Cpu(usize),
    This,
    All,
    AllButThis,
}

#[derive(Debug, Copy, Clone)]
pub enum TaskIpiOperation {
    Queue,
    WakeUp,
    WakeUpNext,
    Continue,
}

intrusive_adapter!(TaskIpiAdapter = Box<TaskIpi> : TaskIpi { link => LinkedListLink });

#[derive(Default)]
struct FlushAllIpi {
    generation: AtomicU64,
    target_gen: PerCpu<AtomicU64>,
    flushed_gen: PerCpu<AtomicU64>,
}

unsafe impl Sync for FlushAllIpi {}

impl FlushAllIpi {
    fn new() -> Self {
        dbgln!(ipi_flush, "ipi flush handler created");
        FlushAllIpi::default()
    }

    /// Run flush all request on all cpus but this one and wait for completion
    fn run(&self) {
        let requested_gen = self.generation.fetch_add(1, Ordering::AcqRel) + 1;

        for target in self.target_gen.iter_all_but_this() {
            target.fetch_max(requested_gen, Ordering::Release);
        }

        dbgln!(ipi_flush, "Running flush all request with gen: {}", requested_gen);

        send_sync_ipi();

        // Spin until all cpus confirm up to this generation
        while self
            .flushed_gen
            .iter_all_but_this()
            .any(|f| {
                f.load(Ordering::Acquire) < requested_gen
            })
        {
            core::hint::spin_loop()
        }

        dbgln!(ipi_flush, "flush all all cpus completed");
    }

    /// Handle flush all request on this cpu
    fn handle(&self) {
        let target_gen = self.target_gen.this_cpu().load(Ordering::Acquire);

        crate::arch::mm::virt::flush_all();

        // Confirm generation for this cpu
        self.flushed_gen.this_cpu().fetch_max(target_gen, Ordering::Release);

        dbgln!(ipi_flush, "handle flush all store gen: {}", target_gen);
    }
}

struct TaskIpi {
    cmd: TaskIpiOperation,
    task: ArcTask,
    link: LinkedListLink,
}

impl TaskIpi {
    fn new(cmd: TaskIpiOperation, task: ArcTask) -> Box<TaskIpi> {
        Box::new(TaskIpi {
            cmd,
            task,
            link: LinkedListLink::new(),
        })
    }
}

struct IpiContext {
    task_ipi: PerCpu<Spin<LinkedList<TaskIpiAdapter>>>,
}

unsafe impl Sync for IpiContext {}
unsafe impl Send for IpiContext {}

impl IpiContext {
    pub fn new() -> Self {
        IpiContext {
            task_ipi: PerCpu::new_fn(|| Spin::new(LinkedList::new(TaskIpiAdapter::new()))),
        }
    }
}

static CONTEXT: Once<IpiContext> = Once::new();
static FLUSH_ALL: Once<FlushAllIpi> = Once::new();

fn context() -> &'static IpiContext {
    unsafe { CONTEXT.get_unchecked() }
}

pub fn init() {
    dbgln!(ipi, "IPI init");
    CONTEXT.call_once(|| IpiContext::new());
    FLUSH_ALL.call_once(|| FlushAllIpi::new());
    crate::arch::ipi::init();
}

pub fn init_ap() {}

pub fn exec_on_cpu(target: IpiTarget, kind: IpiKind) {
    crate::arch::ipi::send_ipi_to(target, kind);
}

pub fn wake_up(task: &ArcTask) {
    let cpu = task.on_cpu();

    if cpu == unsafe { crate::CPU_ID as usize } {
        crate::kernel::sched::internal().wake(task.clone());
    } else {
        dbgln!(ipi, "sending wake {}", task.on_cpu());
        send_task_ipi(TaskIpiOperation::WakeUp, task);
    }
}

pub fn wake_up_next(task: &ArcTask) {
    let cpu = task.on_cpu();

    if cpu == unsafe { crate::CPU_ID as usize } {
        crate::kernel::sched::internal().wake_as_next(task.clone());
    } else {
        send_task_ipi(TaskIpiOperation::WakeUpNext, task);
    }
}

pub fn cont(task: &ArcTask) {
    let cpu = task.on_cpu();

    if cpu == unsafe { crate::CPU_ID as usize } {
        crate::kernel::sched::internal().cont(task.clone());
    } else {
        send_task_ipi(TaskIpiOperation::Continue, task);
    }
}

pub fn queue(task: &ArcTask) {
    let cpu = task.on_cpu();

    if cpu == crate::cpu_id() as usize {
        kernel::sched::internal().queue_task(task.clone(), false);
    } else {
        send_task_ipi(TaskIpiOperation::Queue, task);
    }
}

pub fn tlb_flush_all() {
    unsafe {
        FLUSH_ALL.get_unchecked().run()
    }
}

static COUNT: AtomicU64 = AtomicU64::new(0);

fn handle_ipi_task_exec(cmd: TaskIpiOperation, task: &ArcTask) {
    match cmd {
        TaskIpiOperation::Queue => {
            kernel::sched::internal().queue_task(task.clone(), false);
        }
        TaskIpiOperation::WakeUp => {
            kernel::sched::internal().wake(task.clone());
        }
        TaskIpiOperation::WakeUpNext => {
            kernel::sched::internal().wake_as_next(task.clone());
        }
        TaskIpiOperation::Continue => {
            kernel::sched::internal().cont(task.clone());
        }
    }
}

pub fn handle_ipi_task() {
    dbgln!(ipi2, "got ipi handle {}", crate::cpu_id());
    let ctx = context();

    let lock = ctx.task_ipi.this_cpu();

    let mut cnt = 0;

    let _g = IrqGuard::new();

    let mut locked = lock.lock_irq();

    /* We need a list here in case multiple cpus send ipi to the same core.
     * In this case we would receive only one interrupt.
     * We call end_of_int while holding a lock,
     * which guarantees elements added after while loop finishes will be processed on next interrupt
     */
    while let Some(el) = locked.pop_front() {
        cnt += 1;

        drop(locked);

        //dbgln!(ipie, "handle {:?}", el.cmd);
        handle_ipi_task_exec(el.cmd, &el.task);

        locked = lock.lock_irq();
    }
    drop(locked);

    dbgln!(ipi_count, "handled {} ipis", cnt);
    crate::arch::int::end_of_int();
}

pub fn handle_ipi_sync() {
    unsafe {
        FLUSH_ALL.get_unchecked().handle();
    }
    crate::arch::int::end_of_int();
}

fn send_task_ipi(task_ipi: TaskIpiOperation, task: &ArcTask) {
    let ctx = context();

    let cpu = ctx.task_ipi.cpu(task.on_cpu() as isize);
    {
        let mut lock = cpu.lock_irq();
        lock.push_back(TaskIpi::new(task_ipi, ArcTask::from(task.clone())));
    }

    if unsafe { task.arch_task() }.is_user() {
        dbgln!(ipie, "{} E -> {}", crate::cpu_id(), task.on_cpu());
    }
    dbgln!(
        ipi_count,
        "+ {} {}->{} ({:?})",
        COUNT.fetch_add(1, Ordering::SeqCst) + 1,
        crate::cpu_id(),
        task.on_cpu(),
        task_ipi
    );
    exec_on_cpu(IpiTarget::Cpu(task.on_cpu()), IpiKind::IpiTask);
}

fn send_sync_ipi() {
    exec_on_cpu(IpiTarget::AllButThis, IpiKind::IpiSync);
}

pub fn ipi_test() {
    exec_on_cpu(IpiTarget::All, IpiKind::IpiTest);
}
