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

pub fn cli() {
    unsafe {
        asm!("cli");
    }
}

fn remap_irq(irq: u32) -> u32 {
    ACPI.lock().find_irq_remap(irq)
}

pub fn end_of_int() {
    //ACPI.lock().end_of_int();
    unsafe {
        PIC.lock().notify_end_of_interrupt(32);
    }
}

pub fn init() {
    let cp = &mut *PIC.lock();
    cp.init();
    //cp.disable();
    let acpi = &mut *ACPI.lock();
    acpi.init();
    //let acpi3 = &mut *ACPI.lock();
    //acpi3.init();

    let remap = acpi.find_irq_remap(0);
    acpi.set_int_dest(remap, 32);
        //cp.init_timer(50);
}
