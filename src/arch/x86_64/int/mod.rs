
use spin::Mutex;
use alloc::boxed::Box;

use arch::acpi::Acpi;
use arch::dev::pic::ChainedPics;

static ACPI: Mutex<Acpi> = Mutex::new(Acpi::new());
static PIC: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(0x20, 0x28) });

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
    ACPI.lock().find_irq_remap(irq)
}

pub fn end_of_int() {
    ACPI.lock().end_of_int()
}

pub fn mask_int(int: u8, masked: bool) {
    ACPI.lock().mask_int(int, masked);
}

pub fn set_irq_dest(src: u8, dst: u8) {
    ACPI.lock().set_int_dest(src as u32, dst as u32);
}

pub fn init() {
    let mut pic = PIC.lock();

    pic.init();
    pic.disable();

    let mut apic = ACPI.lock();

    apic.init();
}
