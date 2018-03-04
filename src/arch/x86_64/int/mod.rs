use spin::Mutex;

use arch::acpi::Acpi;

static ACPI: Mutex<Acpi> = Mutex::new(Acpi::new());

pub fn init() {
    let mut acpi = &mut *ACPI.lock();

    acpi.init();
}
