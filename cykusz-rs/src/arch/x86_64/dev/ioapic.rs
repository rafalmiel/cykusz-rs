use core::ptr::read_volatile;
use core::ptr::write_volatile;

use bit_field::BitField;

use crate::arch::acpi::apic::MadtHeader;
use crate::arch::x86_64::acpi::apic::MadtEntryIntSrc;
use crate::kernel::mm::*;
use crate::kernel::sync::{LockApi, Spin};

pub static IOAPIC: Spin<IOApic> = Spin::new(IOApic::new());

const REG_ID: u32 = 0x00;
const REG_VER: u32 = 0x01;
const REG_ARB: u32 = 0x02;

const fn reg_redtbl_low(num: u32) -> u32 {
    0x10 + (2 * num)
}

const fn reg_redtbl_high(num: u32) -> u32 {
    0x11 + (2 * num)
}

struct RegId(u32);

struct RegVer(u32);

struct RegRedTblL(u32);

struct RegRedTblH(u32);

impl RegId {
    pub const fn id(&self) -> u32 {
        (self.0 >> 24) & 0b1111
    }
}

impl RegVer {
    pub const fn apic_version(&self) -> u32 {
        self.0 & 0xFF
    }

    pub const fn max_red_entry(&self) -> u32 {
        (self.0 >> 16) & 0xFF
    }
}

impl RegRedTblL {
    #[allow(unused)]
    pub const fn masked(&self) -> bool {
        self.0 & (1 << 16) != 0
    }

    pub fn set_masked(&mut self, masked: bool) {
        if masked {
            self.0 |= 1 << 16;
        } else {
            self.0 &= !(1 << 16);
        }
    }

    pub fn set_vector(&mut self, idx: u32) {
        self.0 = (self.0 & !(0xFFu32)) | (idx & 0xFF);
    }

    #[allow(unused)]
    pub const fn vector(&self) -> u32 {
        self.0 & 0xFF
    }
}

impl RegRedTblH {
    pub fn set_destination(&mut self, dest: u32) {
        self.0 = (self.0 & !(0xFFu32 << 24)) | (dest & 0xFF);
    }

    #[allow(unused)]
    pub const fn destination(&mut self) -> u32 {
        self.0 >> 24
    }
}

pub struct IOApic {
    ioapic_base: Option<MappedAddr>,
}

impl IOApic {
    fn read(&self, reg: u32) -> u32 {
        if let Some(base) = self.ioapic_base {
            unsafe {
                write_volatile::<u32>(base.0 as *mut u32, reg);

                return read_volatile::<u32>((base.0 + 0x10) as *const u32);
            }
        } else {
            panic!("IOApic module not initialised");
        }
    }

    fn write(&self, reg: u32, value: u32) {
        if let Some(base) = self.ioapic_base {
            unsafe {
                write_volatile::<u32>(base.0 as *mut u32, reg);
                write_volatile::<u32>((base.0 + 0x10) as *mut u32, value);
            }
        } else {
            panic!("IOApic module not initialised");
        }
    }

    pub fn id(&self) -> u32 {
        RegId(self.read(REG_ID)).id()
    }

    pub fn identification(&self) -> u32 {
        RegId(self.read(REG_ARB)).id()
    }

    pub fn max_red_entry(&self) -> u32 {
        RegVer(self.read(REG_VER)).max_red_entry()
    }

    pub fn version(&self) -> u32 {
        RegVer(self.read(REG_VER)).apic_version()
    }

    pub fn mask_int(&mut self, mut i: u32, masked: bool, ovride: Option<&'static MadtEntryIntSrc>) {
        if let Some(ent) = ovride {
            i = ent.global_sys_int();
        }

        let mut l = RegRedTblL(self.read(reg_redtbl_low(i)));
        let h = RegRedTblH(self.read(reg_redtbl_high(i)));

        l.set_masked(masked);

        self.write(reg_redtbl_low(i), l.0);
        self.write(reg_redtbl_high(i), h.0);
    }

    pub fn set_int(&mut self, mut src: u32, dest: u32, ovride: Option<&'static MadtEntryIntSrc>) {
        if let Some(ent) = ovride {
            src = ent.global_sys_int();
        }

        let mut l = RegRedTblL(self.read(reg_redtbl_low(src)));
        let mut h = RegRedTblH(self.read(reg_redtbl_high(src)));

        l.0 = 0;
        h.0 = 0;

        l.set_vector(dest);
        l.set_masked(false);
        h.set_destination(self.id());

        if let Some(ent) = ovride {
            if ent.active_low() {
                l.0 |= 1 << 13; //active low
            }
            if ent.level_triggered() {
                l.0 |= 1 << 15; //level triggered
            }
        }

        self.write(reg_redtbl_low(src), l.0);
        self.write(reg_redtbl_high(src), h.0);
    }

    pub fn set_int_active_high(&mut self, src: u32, val: bool) {
        let mut l = RegRedTblL(self.read(reg_redtbl_low(src)));
        let h = RegRedTblH(self.read(reg_redtbl_high(src)));

        l.0.set_bit(13, !val);

        self.write(reg_redtbl_low(src), l.0);
        self.write(reg_redtbl_high(src), h.0);
    }

    pub fn set_int_level_triggered(&mut self, src: u32, val: bool) {
        let mut l = RegRedTblL(self.read(reg_redtbl_low(src)));
        let h = RegRedTblH(self.read(reg_redtbl_high(src)));

        l.0.set_bit(15, val);

        self.write(reg_redtbl_low(src), l.0);
        self.write(reg_redtbl_high(src), h.0);
    }

    pub const fn new() -> IOApic {
        IOApic { ioapic_base: None }
    }

    pub fn init(&mut self, hdr: &'static MadtHeader) {
        if let Some(ref io) = hdr.ioapic_entries().nth(0) {
            self.ioapic_base = Some(io.ioapic_address());
        } else {
            panic!("IOApic could not be initialized");
        }

        for i in 0..self.max_red_entry() + 1 {
            self.mask_int(i, true, None);
        }
    }
}

pub fn init(hdr: &'static MadtHeader) {
    IOAPIC.lock().init(hdr);
}
