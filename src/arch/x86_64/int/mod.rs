
use spin::Mutex;
use alloc::boxed::Box;

use arch::acpi::Acpi;
use arch::dev::pic::ChainedPics;


pub trait InterruptController : Send + Sync {
    fn init(&mut self);
    fn end_of_int(&mut self);
    fn irq_remap(&self, irq: u32) -> u32;
    fn mask_int(&mut self, int: u8, masked: bool);
    fn disable(&mut self);
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

pub fn remap_irq(irq: u32) -> u32 {
    ::arch::acpi::ACPI.lock().find_irq_remap(irq)
}

pub fn end_of_int() {
    ::arch::dev::lapic::LAPIC.lock().end_of_int()
}

pub fn mask_int(int: u8, masked: bool) {
    ::arch::dev::ioapic::IOAPIC.lock().mask_int(int as u32, masked);
}

pub fn set_irq_dest(src: u8, dst: u8) {
    ::arch::dev::ioapic::IOAPIC.lock().set_int(src as u32, dst as u32);
}

