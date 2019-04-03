pub mod cpu;
pub mod hpet;
pub mod ioapic;
pub mod lapic;
pub mod pic;
pub mod pit;
pub mod rtc;

pub fn init() {
    pic::init();
    if let Some(ref apic) = crate::arch::acpi::ACPI.lock().get_apic_entry() {
        //We have local apic, so disable PIC
        pic::disable();

        println!("[ OK ] PIC Disabled");

        ioapic::init(apic);

        println!("[ OK ] IOAPIC Initialized");

        lapic::init(apic);

        println!("[ OK ] LAPIC Initialized (x2apic: {})", cpu::has_x2apic());
    } else {
        panic!("No APIC found!");
    }

    // initialise and disable pit. its used to implement busy sleep
    // sleep function mask and unmask interrupts when needed
    pit::init();
    pit::disable();

    println!("[ OK ] PIT Disabled");
}

pub fn init_ap() {
    lapic::init_ap();
}
