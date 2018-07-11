pub trait InterruptController : Send + Sync {
    fn init(&mut self);
    fn end_of_int(&mut self);
    fn irq_remap(&self, irq: u32) -> u32;
    fn mask_int(&mut self, int: u8, masked: bool);
    fn disable(&mut self);
}

pub fn is_int_enabled() -> bool {
    unsafe {
        let r: usize;
        asm!("pushfq; popq $0" : "=r"(r) :: "memory");
        return (r & (1usize << 9)) > 0;
    }
}

pub fn sti() {
    unsafe {
        asm!("sti");
    }
}

pub fn cli() {
    unsafe {
        asm!("cli");
    }
}

pub fn get_irq_mapping(irq: u32) -> u32 {
    crate::arch::acpi::ACPI.lock().get_irq_mapping(irq)
}

pub fn end_of_int() {
    crate::arch::dev::lapic::LAPIC.irq().end_of_int()
}

pub fn mask_int(int: u8, masked: bool) {
    crate::arch::dev::ioapic::IOAPIC.lock().mask_int(int as u32, masked);
}

pub fn set_irq_dest(src: u8, dst: u8) {
    crate::arch::dev::ioapic::IOAPIC.lock().set_int(src as u32, dst as u32);
}
