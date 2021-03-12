pub const IPI_VECTOR: u8 = 0x82;

pub fn init() {
    crate::arch::idt::set_handler(IPI_VECTOR as usize, ipi_interrupt);
}

pub fn send_ipi_to(target: usize) {
    crate::arch::int::send_ipi(target, IPI_VECTOR);
}

fn ipi_interrupt() {
    crate::kernel::ipi::handle_ipi();
}
