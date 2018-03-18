mod rsdp;
mod util;
mod rsdt;
pub mod apic;
pub mod hpet;

use self::rsdp::Address;
use kernel::mm::{PhysAddr,MappedAddr,VirtAddr};
use self::rsdt::Rsdt;

use spin::Mutex;

pub static ACPI: Mutex<Acpi> = Mutex::new(Acpi::new());

pub struct Acpi {
    rsdt: Option<&'static Rsdt>,
}

impl Acpi {
    pub const fn new() -> Acpi {
        Acpi {
            rsdt: None,
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
    }

    pub fn get_rsdt(&self) -> Option<&'static Rsdt> {
        self.rsdt
    }

    pub fn get_irq_mapping(&mut self, irq: u32) -> u32 {
        let apic = self.rsdt.expect("RSDT not initialized").find_apic_entry().expect("APIC Entry Not Found!");
        apic.find_irq_remap(irq)
    }

    pub fn has_hpet(&self) -> bool {
  //      self.hpet.is_some()
        true
    }
}

pub fn init() {
    let acpi = &mut *ACPI.lock();
    acpi.init();

    println!("[ OK ] ACPI found...? {}", if acpi.get_rsdt().is_some() { "YES" } else { "NO" });
}