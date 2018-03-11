mod rsdp;
mod util;
mod rsdt;
mod apic;
pub mod hpet;

use self::rsdp::Address;
use kernel::mm::{PhysAddr,MappedAddr,VirtAddr};
use self::rsdt::Rsdt;
use arch::dev::lapic::LApic;
use arch::dev::ioapic::IOApic;
use arch::dev::hpet::Hpet;
use self::hpet::HpetHeader;

pub struct Acpi {
    rsdt: Option<&'static Rsdt>,
    lapic: LApic,
    ioapic: IOApic,
    hpet: Option<Hpet>,
}

impl Acpi {
    pub const fn new() -> Acpi {
        Acpi {
            rsdt: None,
            lapic: LApic::new(),
            ioapic: IOApic::new(),
            hpet: None
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

        if let Some(h) = self.rsdt.expect("RSDT").find_hpet_entry() {
            //println!("HPET 0x{:x} 0x{:x} 0x{:x}", h.address, h.minimum_tick, h.pci_vendor_id);

            self.hpet = Some(Hpet::new(h));
        } else {
            println!("HPET not found, switching back to PIT");
        }
        if let Some(ref ht) = self.hpet {
            println!("HPET Tick Period: {}", ht.counter_clk_period());
        }

    }

    pub fn mask_int(&mut self, idx: u8, masked: bool) {
        self.ioapic.mask_int(idx as u32, masked);
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

    pub fn has_hpet(&self) -> bool {
        self.hpet.is_some()
    }
}