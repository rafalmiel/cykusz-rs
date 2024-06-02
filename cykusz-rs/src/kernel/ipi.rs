use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Once;

use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};
use crate::kernel::utils::PerCpu;

type IpiFn = fn(usize, *const ());

pub struct IpiCommand {
    fun: IpiFn,
    arg: *const (),
    src_cpu: usize,

    completed: AtomicUsize,
    wq: WaitQueue,
}

impl IpiCommand {
    pub fn new<T: Sized>(fun: IpiFn, arg: Option<&T>) -> Arc<IpiCommand> {
        Arc::new(IpiCommand {
            fun,
            arg: if let Some(a) = arg {
                a as *const _ as *const ()
            } else {
                core::ptr::null()
            },
            src_cpu: unsafe { crate::CPU_ID } as usize,
            completed: AtomicUsize::new(0),
            wq: WaitQueue::new(),
        })
    }

    fn completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    fn inc_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }

    fn mark_complete(&self) {
        self.inc_completed();

        self.wq.notify_one();
    }

    fn execute(&self) {
        (self.fun)(self.src_cpu, self.arg);

        self.mark_complete();
    }

    fn wait_complete(&self) {
        self.wq
            .wait_for(
                WaitQueueFlags::NON_INTERRUPTIBLE | WaitQueueFlags::IRQ_DISABLE,
                || self.completed() == 1,
            )
            .expect("not interruptble wait interrupted");
    }
}

unsafe impl Send for Ipi {}
unsafe impl Sync for Ipi {}

struct IpiCommandList {
    ipis: Spin<LinkedList<Arc<IpiCommand>>>,
    wq: WaitQueue,
}

impl IpiCommandList {
    pub fn new() -> IpiCommandList {
        IpiCommandList {
            ipis: Spin::new(LinkedList::new()),
            wq: WaitQueue::new(),
        }
    }
}

pub struct Ipi {
    ipis: PerCpu<IpiCommandList>,
}

impl Ipi {
    pub fn new() -> Ipi {
        Ipi {
            ipis: PerCpu::new_fn(|| IpiCommandList::new()),
        }
    }
}

static IPI: Once<Ipi> = Once::new();

fn ipi() -> &'static Ipi {
    IPI.get().unwrap()
}

pub fn init() {
    IPI.call_once(|| Ipi::new());

    crate::arch::ipi::init();

    init_ap();
}

pub fn init_ap() {
    crate::kernel::sched::create_task(ipi_thread);
}

pub fn exec_on_cpu(target: usize, cmd: Arc<IpiCommand>) {
    let ipi = ipi();

    let ipis = ipi.ipis.cpu(target as isize);

    let mut list = ipis.ipis.lock();

    list.push_back(cmd.clone());

    drop(list);

    crate::arch::ipi::send_ipi_to(target);

    cmd.wait_complete();
}

fn ipi_thread() {
    // Thread context for this CPU execution
    let ipi = ipi().ipis.this_cpu();

    loop {
        let mut list = ipi
            .wq
            .wait_lock_for(WaitQueueFlags::NON_INTERRUPTIBLE, &ipi.ipis, |l| {
                !l.is_empty()
            })
            .expect("ipi_thread should not be signalled")
            .unwrap();

        while let Some(cmd) = list.pop_front() {
            cmd.execute();
        }
    }
}

pub fn handle_ipi() {
    let ipi = ipi().ipis.this_cpu();

    // Notify IPI thread as we don't want to exec ipis in the interrupt context
    ipi.wq.notify_one();
}

fn ipi_test(src: usize, arg: *const ()) {
    println!(
        "[ IPI ] Exec ipi from cpu {} on cpu {}, msg: {}",
        src,
        unsafe { crate::CPU_ID },
        unsafe { (arg as *const usize).read() }
    );
}

pub fn test_ipi() {
    let count = crate::kernel::smp::cpu_count();

    if count > 1 {
        println!("[ IPI ] Self test, send ipi to cpu 1");
        let msg = 42usize;

        exec_on_cpu(1, IpiCommand::new(ipi_test, Some(&msg)));
    }
}
