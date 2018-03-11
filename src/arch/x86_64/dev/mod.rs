pub mod pic;
pub mod rtc;
pub mod pit;
pub mod ioapic;
pub mod lapic;
pub mod hpet;

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

            println!("[ OK ] LAPIC Initialized");
        }
    }
    rtc::init();

    println!("[ OK ] RTC Initialized");
}