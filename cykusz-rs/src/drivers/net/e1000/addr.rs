use crate::arch::raw::cpuio::Port;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::net::e1000::regs::Regs;

pub struct Addr {
    mmio: bool,
    base: u64,
}

impl Addr {
    pub fn new() -> Addr {
        Addr {
            mmio: false,
            base: 0,
        }
    }

    pub fn base(&self) -> u64 {
        self.base
    }

    pub fn read(&self, reg: Regs) -> u32 {
        self.read_raw(reg as u32)
    }

    pub fn read_raw(&self, reg: u32) -> u32 {
        if self.mmio {
            unsafe {
                return PhysAddr(self.base as usize + reg as usize)
                    .to_mapped()
                    .read_volatile::<u32>();
            }
        } else {
            unsafe {
                Port::<u32>::new(self.base as u16).write(reg as u32);
                return Port::<u32>::new(self.base as u16 + 0x4).read();
            }
        }
    }

    pub fn write(&self, reg: Regs, val: u32) {
        self.write_raw(reg as u32, val);
    }

    pub fn write_raw(&self, reg: u32, val: u32) {
        if self.mmio {
            unsafe {
                return PhysAddr(self.base as usize + reg as usize)
                    .to_mapped()
                    .store_volatile(val);
            }
        } else {
            unsafe {
                Port::<u32>::new(self.base as u16).write(reg as u32);
                Port::<u32>::new(self.base as u16 + 0x4).write(val);
            }
        }
    }

    pub fn flag(&self, reg: Regs, flag: u32, value: bool) {
        self.flag_raw(reg as u32, flag, value);
    }

    pub fn flag_raw(&self, reg: u32, flag: u32, value: bool) {
        if value {
            self.write_raw(reg, self.read_raw(reg) | flag);
        } else {
            self.write_raw(reg, self.read_raw(reg) & !flag);
        }
    }

    pub fn init(&mut self, bar0: u32, bar1: u32) {
        self.mmio = (bar0 & 0b1) == 0;

        if self.mmio {
            if (bar0 >> 1) & 0b11 == 2 {
                self.base = (bar0 as u64 & 0xffff_fff0) + ((bar1 as u64) << 32);
            } else {
                self.base = bar0 as u64 & 0xffff_fff0;
            }
        } else {
            self.base = bar0 as u64 & 0xffff_fffc;
        }
    }
}
