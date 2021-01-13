pub mod cpu;
pub mod hpet;
pub mod ioapic;
pub mod lapic;
pub mod pic;
pub mod pit;
pub mod rtc;
pub mod serial;

pub fn init() {
    pic::init();
    if let Some(apic) = crate::arch::acpi::ACPI.lock().get_apic_entry() {
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

    if let Some(ref hpet) = crate::arch::acpi::ACPI.lock().get_hpet_entry() {
        hpet::init(hpet);

        println!("[ OK ] HPET Enabled")
    } else {
        panic!("[ ERROR ] HPET Not found");
    }

    println!("[ OK ] PIT Disabled");

    rtc::init();

    println!("[ OK ] RTC Enabled")
}

pub fn init_ap() {
    lapic::init_ap();
}
