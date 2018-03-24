pub mod pic;
pub mod rtc;
pub mod pit;
pub mod ioapic;
pub mod lapic;
pub mod hpet;
pub mod cpu;

pub fn init()
{
    pic::init();
    if let Some(ref rsdt) = ::arch::acpi::ACPI.lock().get_rsdt() {
        if let Some(ref apic) = rsdt.find_apic_entry() {
            //We have local apic, so disable PIC
            pic::disable();

            println!("[ OK ] PIC Disabled");

            ioapic::init(apic);

            println!("[ OK ] IOAPIC Initialized");

            lapic::init(apic);

            println!("[ OK ] LAPIC Initialized (x2apic: {})", cpu::has_x2apic());
        }
    }

    // initialise and disable pit. its used to implement busy sleep
    // sleep function mask and unmask interrupts when needed
    pit::init();
    pit::disable();

    println!("[ OK ] PIT Disabled");
}

pub fn init_ap()
{
    if let Some(ref rsdt) = ::arch::acpi::ACPI.lock().get_rsdt() {
        if let Some(ref apic) = rsdt.find_apic_entry() {
            lapic::init_ap(apic);
        }
    }
}