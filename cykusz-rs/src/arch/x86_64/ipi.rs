use crate::kernel::ipi;
use crate::kernel::ipi::IpiTarget;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum IpiKind {
    IpiAsync = 82,
    IpiTest = 83,
    IpiSync = 84,
}

impl IpiTarget {
    pub fn get_dest_target(&self) -> (usize, usize) {
        match self {
            IpiTarget::Cpu(t) => (0, *t),
            IpiTarget::This => (1, 0),
            IpiTarget::All => (2, 0),
            IpiTarget::AllButThis => (3, 0),
        }
    }
}

pub fn init() {
    crate::arch::idt::set_handler(IpiKind::IpiAsync as usize, ipi_async);
    crate::arch::idt::set_handler(IpiKind::IpiSync as usize, ipi_sync);
    crate::arch::idt::set_handler(IpiKind::IpiTest as usize, ipi_test);
}

pub fn send_ipi_to(target: ipi::IpiTarget, kind: IpiKind) {
    crate::arch::int::send_ipi(target, kind as u8);
}

fn ipi_async() {
    ipi::r#async::handle_async_ipi();
}

fn ipi_sync() {
    ipi::sync::handle_sync_ipi();
}

fn ipi_test() {
    dbgln!(ipi, "ipi on cpu {}", crate::cpu_id());
}
