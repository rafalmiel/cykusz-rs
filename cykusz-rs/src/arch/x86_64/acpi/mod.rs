use crate::arch::x86_64::acpi::apic::MadtEntryIntSrc;
use crate::kernel::sync::{LockApi, Spin};

use self::rsdp::Address;
use self::rsdt::Rsdt;

pub mod apic;
pub mod hpet;
mod rsdp;
mod rsdt;
mod util;

pub static ACPI: Spin<Acpi> = Spin::new(Acpi::new());

enum Header {
    RSDT(Option<&'static Rsdt<u32>>),
    XSDT(Option<&'static Rsdt<u64>>),
    None,
}

pub struct Acpi {
    hdr: Header,
}

impl Acpi {
    pub const fn new() -> Acpi {
        Acpi { hdr: Header::None }
    }

    pub fn init(&mut self) -> bool {
        let rsdt = rsdp::find_rsdt_address().expect("RSDT Addr Not Found!");

        match rsdt {
            Address::Rsdp(addr) => {
                println!("[ OK ] ACPI Found Rsdp Header");
                self.hdr = Header::RSDT(Some(Rsdt::<u32>::new(addr)))
            }
            Address::Xsdp(addr) => {
                println!("[ OK ] ACPI Found Xsdp Header");
                self.hdr = Header::XSDT(Some(Rsdt::<u64>::new(addr)))
            }
        };
        return true;
    }

    pub fn get_apic_entry(&self) -> Option<&'static apic::MadtHeader> {
        match self.hdr {
            Header::RSDT(ref r) => r.unwrap().find_apic_entry(),
            Header::XSDT(ref r) => r.unwrap().find_apic_entry(),
            _ => {
                panic!("ACPI Not Initialised");
            }
        }
    }

    pub fn get_hpet_entry(&self) -> Option<&'static hpet::HpetHeader> {
        match self.hdr {
            Header::RSDT(ref r) => r.unwrap().find_hpet_entry(),
            Header::XSDT(ref r) => r.unwrap().find_hpet_entry(),
            _ => {
                panic!("ACPI Not Initialised");
            }
        }
    }

    pub fn get_mcfg_entry(&self) -> Option<&'static acpica::acpi_table_mcfg> {
        match self.hdr {
            Header::RSDT(ref r) => r.unwrap().find_mcfg_entry(),
            Header::XSDT(ref r) => r.unwrap().find_mcfg_entry(),
            _ => panic!("ACPI Not Initialised"),
        }
    }

    pub fn print_tables(&self) {
        match self.hdr {
            Header::RSDT(ref r) => r.unwrap().print_tables(),
            Header::XSDT(ref r) => r.unwrap().print_tables(),
            _ => {
                panic!("ACPI Not Initialised");
            }
        }
    }

    pub fn get_irq_mapping(&mut self, irq: u32) -> Option<&'static MadtEntryIntSrc> {
        let apic = self.get_apic_entry().expect("APIC Entry not found");
        apic.find_irq_remap(irq)
    }

    pub fn debug_redirection_entries(&mut self) {
        let apic = self.get_apic_entry().expect("APIC Entry not found");

        let red = apic.intsrc_entries();

        for e in red {
            println!(
                "{} {} {} {}",
                e.irq_src(),
                e.global_sys_int(),
                e.active_low(),
                e.level_triggered()
            );
        }
    }

    pub fn has_hpet(&self) -> bool {
        //      self.hpet.is_some()
        true
    }
}

pub fn init() {
    let acpi = &mut *ACPI.lock();
    let res = acpi.init();

    println!("[ OK ] ACPI Found...? {}", if res { "YES" } else { "NO" });
}

pub fn power_off() -> ! {
    unsafe {
        acpica::AcpiEnterSleepStatePrep(5);
        crate::kernel::int::disable();
        acpica::AcpiEnterSleepState(5);
    }

    panic!("power off failed");
}

pub fn reboot() -> bool {
    unsafe {
        if acpica::AcpiReset() != acpica::AE_OK {
            return false;
        }

        return true;
    }
}
