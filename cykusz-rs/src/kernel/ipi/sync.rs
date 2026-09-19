use crate::kernel::ipi;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::PerCpu;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use intrusive_collections::{LinkedList, LinkedListLink, UnsafeRef};
use spin::Once;

intrusive_adapter!(SyncIpiWorkAdapter = UnsafeRef<SyncIpiWork>: SyncIpiWork { link => LinkedListLink });

pub type SyncIpiArg = *mut ();
pub type SyncIpiFun = fn(SyncIpiArg);

pub enum SyncIpiOp {
    FlushTlbAll,
    Run(SyncIpiFun, SyncIpiArg),
}

/// This is optimized flush all sync ipi handler relying on generation number
#[derive(Default)]
struct FlushAllIpiHandler {
    generation: AtomicU64,
    target_gen: PerCpu<AtomicU64>,
    flushed_gen: PerCpu<AtomicU64>,
}

unsafe impl Sync for FlushAllIpiHandler {}

impl FlushAllIpiHandler {
    fn new() -> Self {
        dbgln!(ipi_flush, "ipi flush handler created");
        FlushAllIpiHandler::default()
    }

    fn has_pending(&self) -> bool {
        let target = self.target_gen.this_cpu().load(Ordering::Acquire);
        let flushed = self.flushed_gen.this_cpu().load(Ordering::Acquire);

        target > flushed
    }

    /// Run flush all request on all cpus but this one and wait for completion
    fn run(&self, target: ipi::IpiTarget) {
        let requested_gen = self.generation.fetch_add(1, Ordering::AcqRel) + 1;

        for target in self.target_gen.iter_ipi_target(target) {
            target.fetch_max(requested_gen, Ordering::Release);
        }

        dbgln!(
            ipi_flush,
            "Running flush all request with gen: {}",
            requested_gen
        );

        ipi::send_ipi_to_target(target, ipi::IpiKind::IpiSync);

        // Spin until all cpus confirm up to this generation
        while self
            .flushed_gen
            .iter_ipi_target(target)
            .any(|f| f.load(Ordering::Acquire) < requested_gen)
        {
            core::hint::spin_loop()
        }

        dbgln!(ipi_flush, "flush all all cpus completed");
    }

    /// Handle flush all request on this cpu
    fn handle(&self) {
        if !self.has_pending() {
            return;
        }

        let target_gen = self.target_gen.this_cpu().load(Ordering::Acquire);

        crate::arch::mm::virt::flush_all();

        // Confirm generation for this cpu
        self.flushed_gen
            .this_cpu()
            .fetch_max(target_gen, Ordering::Release);

        dbgln!(ipi_flush, "handle flush all store gen: {}", target_gen);
    }
}

static FLUSH_ALL_HANDLER: Once<FlushAllIpiHandler> = Once::new();
fn flush_all_ipi_handler() -> &'static FlushAllIpiHandler {
    unsafe { FLUSH_ALL_HANDLER.get_unchecked() }
}

struct SyncIpiWork {
    function: SyncIpiFun,
    arg: SyncIpiArg,
    done: *const AtomicUsize,
    link: LinkedListLink,
}

unsafe impl Sync for SyncIpiWork {}

impl SyncIpiWork {
    fn new(fun: SyncIpiFun, args: SyncIpiArg, counter: &AtomicUsize) -> Self {
        Self {
            function: fun,
            arg: args,
            done: counter,
            link: LinkedListLink::new(),
        }
    }

    fn do_call(&self) {
        (self.function)(self.arg);
    }

    fn handle(&self) {
        self.do_call();

        unsafe {
            self.done.as_ref_unchecked().fetch_add(1, Ordering::Release);
        }
    }
}

struct SyncIpiHandler {
    cpus: PerCpu<Spin<LinkedList<SyncIpiWorkAdapter>>>,
}

unsafe impl Sync for SyncIpiHandler {}
unsafe impl Send for SyncIpiHandler {}

/// Optimisation to store small number of elements on the stack
/// Using SmallVec unconditionally does not work
/// since reallocation would make stack references invalid
enum WorkVec {
    Stack(smallvec::SmallVec<[SyncIpiWork; 16]>),
    Heap(Vec<SyncIpiWork>),
}

impl WorkVec {
    fn new(capacity: usize) -> WorkVec {
        if capacity > 16 {
            // Preallocate to avoid reallocations
            WorkVec::Heap(Vec::with_capacity(capacity))
        } else {
            WorkVec::Stack(smallvec::SmallVec::new())
        }
    }

    fn push(&mut self, item: SyncIpiWork) {
        match self {
            WorkVec::Stack(v) => v.push(item),
            WorkVec::Heap(v) => v.push(item),
        }
    }

    fn last(&self) -> Option<&SyncIpiWork> {
        match self {
            WorkVec::Stack(v) => v.last(),
            WorkVec::Heap(v) => v.last(),
        }
    }
}

impl SyncIpiHandler {
    fn new() -> Self {
        Self {
            cpus: PerCpu::new_fn(|_| Spin::new(LinkedList::new(SyncIpiWorkAdapter::new()))),
        }
    }

    fn run(&self, target: ipi::IpiTarget, f: SyncIpiFun, arg: SyncIpiArg) {
        let mut count = 0;

        let done_counter = AtomicUsize::new(0);
        let mut works = WorkVec::new(target.cpu_count());

        for cpu in self.cpus.iter_ipi_target(target) {
            // Here we push to WorkVec which is pre-allocated to avoid reallocations
            works.push(SyncIpiWork::new(f, arg, &done_counter));

            let mut lock = cpu.lock_irq();
            lock.push_back(unsafe { UnsafeRef::from_raw(works.last().unwrap()) });
            count += 1;
        }

        ipi::send_ipi_to_target(target, ipi::IpiKind::IpiSync);

        while done_counter.load(Ordering::Acquire) < count {
            core::hint::spin_loop()
        }
    }

    fn handle(&self) {
        let lock = self.cpus.this_cpu();

        let mut guard = lock.lock_irq();
        while let Some(work) = guard.pop_front() {
            drop(guard);

            work.as_ref().handle();

            guard = lock.lock_irq();
        }
    }
}

static SYNC_IPI_HANDLER: Once<SyncIpiHandler> = Once::new();
fn sync_ipi_handler() -> &'static SyncIpiHandler {
    unsafe { SYNC_IPI_HANDLER.get_unchecked() }
}

pub fn handle_sync_ipi() {
    flush_all_ipi_handler().handle();
    sync_ipi_handler().handle();
}

pub fn call_sync(target: ipi::IpiTarget, op: SyncIpiOp) {
    // this cannot be called with interrupts disabled...
    assert!(crate::kernel::int::is_enabled());
    match op {
        SyncIpiOp::FlushTlbAll => flush_all_ipi_handler().run(target),
        SyncIpiOp::Run(fun, args) => {
            sync_ipi_handler().run(target, fun, args);
        }
    }
}

pub fn init() {
    FLUSH_ALL_HANDLER.call_once(|| FlushAllIpiHandler::new());
    SYNC_IPI_HANDLER.call_once(|| SyncIpiHandler::new());
}
