use crate::kernel::sync::Spin;

use self::rsdp::Address;
use self::rsdt::Rsdt;
use crate::kernel::int::disable;
use bitflags::_core::mem::MaybeUninit;

pub mod apic;
pub mod hpet;
mod rsdp;
mod rsdt;
mod util;
mod os;

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

    pub fn get_apic_entry(&self) -> Option<&'static apic::MatdHeader> {
        match self.hdr {
            Header::RSDT(ref r) => r.unwrap().find_apic_entry(),
            Header::XSDT(ref r) => r.unwrap().find_apic_entry(),
            _ => {
                panic!("ACPI Not Initialised");
            }
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

    pub fn get_irq_mapping(&mut self, irq: u32) -> u32 {
        let apic = self.get_apic_entry().expect("APIC Entry not found");
        apic.find_irq_remap(irq)
    }

    pub fn has_hpet(&self) -> bool {
        //      self.hpet.is_some()
        true
    }
}

pub fn init() {
    let acpi = &mut *ACPI.lock();
    let res = acpi.init();

    acpi.print_tables();
    //loop{}

    println!("[ OK ] ACPI Found...? {}", if res { "YES" } else { "NO" });
}

pub fn init_mem() {

    unsafe {
        //acpica::AcpiInitializeSubsystem();
        //println!("init tables: {}", acpica::AcpiInitializeTables(0 as *mut _, 16, false));
        //acpica::AcpiLoadTables();
        //acpica::AcpiEnableSubsystem(0);

        //acpica::AcpiEnterSleepStatePrep(5);
        //disable();
        //acpica::AcpiEnterSleepState(5);
        //panic!("power off");

        //let mut hdr: *mut acpica::ACPI_TABLE_HEADER = core::ptr::null_mut();

        //println!("{}", acpica::AcpiGetTable(b"HPET".as_ptr() as *mut i8, 1, &mut hdr as *mut *mut acpica::ACPI_TABLE_HEADER));

        //println!("{:?}", unsafe {
        //    *hdr
        //});
    }

}
