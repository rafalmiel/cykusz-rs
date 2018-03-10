mod rsdp;
mod util;
mod rsdt;
mod apic;

use self::rsdp::Address;
use kernel::mm::{PhysAddr,MappedAddr,VirtAddr};
use self::rsdt::Rsdt;

pub struct Acpi {
    rsdt: Option<&'static Rsdt>,
    lapic: apic::lapic::LApic,
    ioapic: apic::ioapic::IOApic
}

impl Acpi {
    pub const fn new() -> Acpi {
        Acpi {
            rsdt: None,
            lapic: apic::lapic::LApic::new(),
            ioapic: apic::ioapic::IOApic::new(),
        }
    }

    pub fn init(&mut self) {
        let rsdt = rsdp::find_rsdt_address().expect("RSDT Addr Not Found!");

        let rsdt_addr = match rsdt {
            Address::Rsdp(addr) => {
                addr
            },
            Address::Xsdp(addr) => {
                panic!("Xsdp address is not yet supported!")
            }
        };

        self.rsdt = Some(Rsdt::new(rsdt_addr));

        let apic = self.rsdt.expect("RSDT not initialized").find_apic_entry().expect("APIC Entry Not Found!");

        for l in apic.lapic_entries() {
            println!("LAPIC: {} {}", l.apic_id, l.proc_id);
        }

        for io in apic.ioapic_entries() {
            println!("IOAPIC: 0x{:x} {} {}", io.ioapic_address, io.ioapic_id, io.global_int_base);
        }

        self.lapic.init(apic.lapic_address());

        for io in apic.ioapic_entries() {
            self.ioapic.init(io.ioapic_address())
        }
    }

    pub fn mask_int(&mut self, idx: u32, masked: bool) {
        self.ioapic.mask_int(idx, masked);
    }

    pub fn set_int_dest(&mut self, idx: u32, dest: u32) {
        self.ioapic.set_int(idx, dest);
    }

    pub fn find_irq_remap(&mut self, irq: u32) -> u32 {
        let apic = self.rsdt.expect("RSDT not initialized").find_apic_entry().expect("APIC Entry Not Found!");
        apic.find_irq_remap(irq)
    }

    pub fn end_of_int(&mut self) {
        self.lapic.end_of_int();
    }
}