pub mod r#async;
pub mod sync;

use crate::arch::ipi::IpiKind;

#[derive(Copy, Clone, Debug)]
pub enum IpiTarget {
    Cpu(usize),
    This,
    All,
    AllButThis,
}

impl IpiTarget {
    pub fn cpu_count(&self) -> usize {
        match self {
            IpiTarget::AllButThis => crate::kernel::smp::cpu_count() - 1,
            IpiTarget::All => crate::kernel::smp::cpu_count(),
            IpiTarget::This | IpiTarget::Cpu(_) => 1,
        }
    }
}

pub fn init() {
    dbgln!(ipi, "IPI init");
    r#async::init();
    sync::init();
    crate::arch::ipi::init();
}

pub fn init_ap() {}

pub fn send_ipi_to_target(target: IpiTarget, kind: IpiKind) {
    crate::arch::ipi::send_ipi_to(target, kind);
}

fn test_ipi_fun(_args: *mut ()) {
    dbgln!(ipi_test, "test ipi function called");
}

pub fn send_test_ipi() {
    sync::call_sync(
        IpiTarget::AllButThis,
        sync::SyncIpiOp::Run(test_ipi_fun, core::ptr::null_mut()),
    );

    dbgln!(ipi_test, "test ipi done");
}
