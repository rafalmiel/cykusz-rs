use crate::arch::ipi::IpiKind;
use crate::kernel::ipi;
use crate::kernel::sync::{IrqGuard, LockApi, Spin};
use crate::kernel::utils::PerCpu;
use alloc::boxed::Box;
use intrusive_collections::{LinkedList, LinkedListLink};
use spin::Once;

intrusive_adapter!(AsyncIpiWorkAdapter = Box<AsyncIpiWork> : AsyncIpiWork { link => LinkedListLink });

pub type AsyncIpiArg = *mut ();
pub type AsyncIpiFun = fn(AsyncIpiArg);

struct AsyncIpiHandler {
    cpus: PerCpu<Spin<LinkedList<AsyncIpiWorkAdapter>>>,
}

impl AsyncIpiHandler {
    fn new() -> Self {
        Self {
            cpus: PerCpu::new_fn(|_| Spin::new(LinkedList::new(AsyncIpiWorkAdapter::new()))),
        }
    }

    fn setup_ipi_on_cpu(&self, cpu: usize, f: AsyncIpiFun, arg: AsyncIpiArg) -> bool {
        let cpu = self.cpus.cpu(cpu as isize);
        {
            let mut lock = cpu.lock_irq();
            let was_empty = lock.is_empty();

            lock.push_back(AsyncIpiWork::new(f, arg));

            was_empty
        }
    }

    fn handle(&self) {
        let lock = self.cpus.this_cpu();

        let _g = IrqGuard::new();

        let mut locked = lock.lock();

        while let Some(el) = locked.pop_front() {
            drop(locked);

            (el.function)(el.arg);

            locked = lock.lock();
        }
    }
}

unsafe impl Sync for AsyncIpiHandler {}
static ASYNC_IPI_HANDLER: Once<AsyncIpiHandler> = Once::new();

fn async_ipi_handler() -> &'static AsyncIpiHandler {
    unsafe { ASYNC_IPI_HANDLER.get_unchecked() }
}

struct AsyncIpiWork {
    function: AsyncIpiFun,
    arg: AsyncIpiArg,
    link: LinkedListLink,
}

unsafe impl Send for AsyncIpiWork {}
unsafe impl Sync for AsyncIpiWork {}

impl AsyncIpiWork {
    fn new(function: AsyncIpiFun, arg: AsyncIpiArg) -> Box<Self> {
        Box::new(Self {
            function,
            arg,
            link: LinkedListLink::new(),
        })
    }
}

pub fn call_async(target: ipi::IpiTarget, f: AsyncIpiFun, arg: AsyncIpiArg) {
    match target {
        ipi::IpiTarget::This => {
            f(arg);
        }
        ipi::IpiTarget::Cpu(cpu) => {
            if cpu == crate::cpu_id() as usize {
                // Skip ipi if we call on this cpu
                f(arg);
                return;
            }

            let was_empty = async_ipi_handler().setup_ipi_on_cpu(cpu, f, arg);

            if was_empty {
                ipi::send_ipi_to_target(target, IpiKind::IpiAsync);
            }
        }
        // AllButThis/All targets would need other way to pass args,
        // as we can't send same pointers to every cpu
        // (e.g. ArcTask would be created from_raw multiple times - dangerous),
        // Perhaps we would need to accept an argument factory function called for each target,
        // We don't need this targets for now, so panic
        t => panic!("async ipi is not implemented for target {:?}", t),
    }
}

pub fn handle_async_ipi() {
    async_ipi_handler().handle()
}

pub fn init() {
    ASYNC_IPI_HANDLER.call_once(AsyncIpiHandler::new);
}
