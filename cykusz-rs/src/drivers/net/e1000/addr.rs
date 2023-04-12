use crate::arch::mm::VirtAddr;
use crate::arch::raw::cpuio::Port;
use crate::drivers::net::e1000::regs::Regs;
use crate::drivers::pci::BarAddress;

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

    pub fn addr_base(&self) -> VirtAddr {
        assert!(self.mmio);

        VirtAddr(self.base as usize)
    }

    pub fn read(&self, reg: Regs) -> u32 {
        self.read_raw(reg as u32)
    }

    pub fn read_raw(&self, reg: u32) -> u32 {
        if self.mmio {
            unsafe {
                return VirtAddr(self.base as usize + reg as usize).read_volatile::<u32>();
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
                return VirtAddr(self.base as usize + reg as usize).store_volatile(val);
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

    pub fn init(&mut self, bar: BarAddress) {
        self.mmio = !bar.is_io();

        if self.mmio {
            self.base = bar.address_map_virt_num(6).0 as u64;
        } else {
            self.base = bar.io_address() as u64;
        }

        logln!("base: 0x{:x}", self.base);
    }
}
