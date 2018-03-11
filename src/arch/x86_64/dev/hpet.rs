use core::ptr::write_volatile;
use core::ptr::read_volatile;

use kernel::mm::{PhysAddr, MappedAddr};

use arch::acpi::hpet::HpetHeader;

pub struct Hpet {
    hpet_hdr: &'static HpetHeader
}

const CAPABILITIES_OFFSET: u64 = 0;

struct RegCapabilities(u64);

impl RegCapabilities {
    pub fn tick_period(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}

impl Hpet {
    pub const fn new(hdr: &'static HpetHeader) -> Hpet {
        Hpet {
            hpet_hdr: hdr
        }
    }

    pub fn counter_clk_period(&self) -> u32 {
        let addr = PhysAddr(self.hpet_hdr.address as usize).to_mapped();
        RegCapabilities(unsafe { (addr.0 as *const u64).read_volatile() }).tick_period()
    }
}