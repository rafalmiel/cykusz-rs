use crate::arch::dev::ioapic::IOAPIC;
use crate::arch::dev::lapic::LAPIC;
use crate::arch::int::InterruptController;
use crate::arch::x86_64::acpi::ACPI;

pub struct Controller {}

pub static CONTROLLER: Controller = Controller {};

impl InterruptController for Controller {
    fn end_of_int(&self) {
        LAPIC.irq().end_of_int();
    }

    fn mask_int(&self, int: u8, masked: bool) {
        IOAPIC
            .lock()
            .mask_int(int as u32, masked, ACPI.lock().get_irq_mapping(int as u32));
    }

    fn set_irq_dest(&self, src: u8, dst: u8) {
        IOAPIC.lock().set_int(
            src as u32,
            dst as u32,
            ACPI.lock().get_irq_mapping(src as u32),
        );
    }
}
