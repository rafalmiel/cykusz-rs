mod rsdp;
mod rsdt;
mod util;
mod lapic;
mod ioapic;

use arch::acpi::rsdt::Rsdt;
use arch::acpi::lapic::LApic;
use arch::acpi::ioapic::IOApic;
use self::rsdp::Address;
use kernel::mm::{PhysAddr,MappedAddr,VirtAddr};

pub struct Acpi {
    pub rsdt: Rsdt,
    pub lapic: LApic,
    pub ioapic: IOApic
}

impl Acpi {
    pub const fn new() -> Acpi {
        Acpi { rsdt: Rsdt::new(), lapic: LApic::new(), ioapic: IOApic::new() }
    }

    pub fn init(&mut self) {
        unsafe {
            let rsdp = rsdp::find_rsdt_address().expect("RSDP Not found!");
            let rsdp_addr = match rsdp {
                Address::Rsdp(addr) => {
                    addr
                },
                Address::Xsdp(addr) => {
                    panic!("Xsdp address is not yet supported!")
                }
            };

            self.rsdt.init(rsdp_addr);

            let lapic_base = self.rsdt.local_controller_address().expect("LAPIC address not found!");

            self.lapic.init(lapic_base);

            let ioapic_base = self.rsdt.ioapic_address().expect("IOApic address not found!");

            self.ioapic.init(ioapic_base);

            println!("[ OK ] IOApic initialised! id: {}, ident: {}, entries: {}, version: {}",
                             self.ioapic.id(), self.ioapic.identification(),
                             self.ioapic.max_red_entry() + 1, self.ioapic.version());
        }
    }
}
