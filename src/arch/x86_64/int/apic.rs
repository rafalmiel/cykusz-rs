use ::arch::acpi::ACPI;
use ::arch::dev::ioapic::IOAPIC;
use ::arch::dev::lapic::LAPIC;
use ::arch::int::InterruptController;

pub struct Controller {}

pub static CONTROLLER: Controller = Controller {};

impl InterruptController for Controller {
    fn end_of_int(&self) {
        LAPIC.irq().end_of_int();
    }

    fn irq_remap(&self, irq: u32) -> u32 {
        ACPI.lock().get_irq_mapping(irq)
    }

    fn mask_int(&self, int: u8, masked: bool) {
        IOAPIC.lock().mask_int(self.irq_remap(int as u32), masked);
    }

    fn set_irq_dest(&self, src: u8, dst: u8) {
        IOAPIC.lock().set_int(self.irq_remap(src as u32), dst as u32);
    }
}
