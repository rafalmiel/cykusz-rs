use core::arch::asm;

use bit_field::BitField;

use crate::arch::acpi::hpet::HpetHeader;
use crate::kernel::mm::MappedAddr;
use crate::kernel::mm::PhysAddr;
use crate::kernel::sync::Spin;

pub struct Hpet {
    hpet_base: Option<MappedAddr>,
    period: u64,
}

#[allow(unused)]
const CAPABILITIES_OFFSET: usize = 0;
const CONFIG_OFFSET: usize = 0x10;
const MAIN_COUNTER_VALUE_OFFSET: usize = 0xF0;

#[derive(Copy, Clone)]
struct RegCapabilities(u64);

#[derive(Copy, Clone)]
struct RegConfig(u64);

impl RegCapabilities {
    pub fn tick_period(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    #[allow(unused)]
    pub fn timer_count(&self) -> u64 {
        self.0.get_bits(8..=12)
    }

    #[allow(unused)]
    pub fn is64bit_mode(&self) -> bool {
        self.0.get_bit(13)
    }
}

impl RegConfig {
    pub fn set_enabled(&mut self, e: bool) {
        self.0.set_bit(0, e);
    }
}

impl Hpet {
    pub const fn new() -> Hpet {
        Hpet {
            hpet_base: None,
            period: 0,
        }
    }

    pub fn init(&mut self, hdr: &'static HpetHeader) {
        self.hpet_base = Some(PhysAddr(hdr.address as usize).to_mapped());
        self.period = self.counter_clk_period() as u64;
        self.set_enabled(true);
    }

    fn get<T: Copy>(&self, offset: usize) -> T {
        unsafe { (self.hpet_base.unwrap() + offset).read::<T>() }
    }

    fn save<T: Copy>(&self, offset: usize, v: T) {
        unsafe { (self.hpet_base.unwrap() + offset).store(v) }
    }

    pub fn counter_clk_period(&self) -> u32 {
        let reg = self.get::<RegCapabilities>(CAPABILITIES_OFFSET);
        reg.tick_period()
    }

    pub fn set_enabled(&self, e: bool) {
        let mut cfg = RegConfig(0);
        cfg.set_enabled(e);

        self.save(CONFIG_OFFSET, cfg);
    }

    pub fn counter_value(&self) -> u64 {
        self.get::<u64>(MAIN_COUNTER_VALUE_OFFSET)
    }

    pub fn current_ns(&self) -> u64 {
        self.counter_value() * self.period / 1_000_000
    }
}

static HPET: Spin<Hpet> = Spin::new(Hpet::new());

pub fn init(hdr: &'static HpetHeader) {
    let mut l = HPET.lock();

    l.init(hdr);
}

pub fn current_ns() -> u64 {
    HPET.lock_irq().current_ns()
}

pub fn busy_sleep(ns: u64) {
    let c = current_ns() + ns;

    while c > current_ns() {
        unsafe {
            asm!("pause");
        }
    }
}
