use crate::kernel::ipi::IpiTarget;

#[repr(u8)]
pub enum IpiKind {
    IpiTask = 82,
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
    // task ipi handler responsible for calling eoi
    crate::arch::idt::set_handler_eoi(IpiKind::IpiTask as usize);
    crate::arch::idt::set_handler_eoi(IpiKind::IpiSync as usize);
    crate::arch::idt::set_handler(IpiKind::IpiTask as usize, ipi_task);
    crate::arch::idt::set_handler(IpiKind::IpiSync as usize, ipi_sync);
    crate::arch::idt::set_handler(IpiKind::IpiTest as usize, ipi_test);
}

pub fn send_ipi_to(target: IpiTarget, kind: IpiKind) {
    crate::arch::int::send_ipi(target, kind as u8);
}

fn ipi_task() {
    crate::kernel::ipi::handle_ipi_task();
}

fn ipi_sync() {
    crate::kernel::ipi::handle_ipi_sync();
}

fn ipi_test() {
    dbgln!(ipi, "ipi on cpu {}", crate::cpu_id());
}
