mod pic;

use spin::Mutex;

use arch::acpi::Acpi;
use self::pic::ChainedPics;

static ACPI: Mutex<Acpi> = Mutex::new(Acpi::new());
static PIC: Mutex<ChainedPics> = Mutex::new( unsafe { ChainedPics::new(0x20, 0x28) } );

pub fn sti() {
    unsafe {
        asm!("sti");
    }
}

fn remap_irq(irq: u32) -> u32 {
    if let Some(i) = ACPI.lock().rsdt.remap_irq(irq) {
        return i;
    } else {
        panic!("Failed to remap irq!");
    }
}

pub fn end_of_interrupt() {
    ACPI.lock().lapic.end_of_interrupt();
}

pub fn init() {
    let cp = &mut *PIC.lock();
    cp.init();
    cp.disable();

    let acpi = &mut *ACPI.lock();
    acpi.init();

    if let Some(i) = acpi.rsdt.remap_irq(0) {
        acpi.ioapic.set_int(i, 32);
        acpi.lapic.fire_timer();
    }
}
