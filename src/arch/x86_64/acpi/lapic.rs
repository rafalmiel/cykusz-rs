use core::ptr::write_volatile;
use core::ptr::read_volatile;

use arch::mm::MappedAddr;

const REG_TRP: u32 = 0x80;
const REG_LCR: u32 = 0xD0;
const REG_DFR: u32 = 0xE0;
const REG_SIVR: u32 = 0xF0;
const REG_EOI: u32 = 0xB0;

const REG_TIM: u32 = 0x320;
const REG_TIMDIV: u32 = 0x3E0;
const REG_TIMINIT: u32 = 0x380;
const REG_TIMCUR: u32 = 0x390;

pub struct LApic {
    lapic_base: Option<MappedAddr>,
}

impl LApic {
    pub const fn new() -> LApic {
        LApic {
            lapic_base: None
        }
    }

    pub fn reg_write(&self, reg: u32, value: u32) {
        if let Some(base) = self.lapic_base {
            unsafe {
                write_volatile::<u32>((base + reg as usize).0 as *mut u32, value);
            }
        } else {
            panic!("Failed write!");
        }
    }

    pub fn reg_read(&self, reg: u32) -> u32 {
        if let Some(base) = self.lapic_base {
            unsafe {
                read_volatile::<u32>((base + reg as usize).0 as *const u32)
            }
        } else {
            panic!("Failed read!");
        }
    }

    pub fn init(&mut self, base: MappedAddr) {
        self.lapic_base = Some(base);

        // Clear task priority to enable all interrupts
        self.reg_write(REG_TRP, 0);

        // Logical Destination Mode
    	self.reg_write(REG_DFR, 0xffffffff);	// Flat mode
    	self.reg_write(REG_LCR, 0x01000000);	// All cpus use logical id 1

    	// Configure Spurious Interrupt Vector Register
    	self.reg_write(REG_SIVR, 0x100 | 0xff);
    }

    pub fn fire_timer(&mut self) {
        self.reg_write(REG_TIMDIV, 0b1010);
        self.reg_write(REG_TIM, 32 | 0x20000);
        self.reg_write(REG_TIMINIT, 0x100000);
    }

    pub fn end_of_interrupt(&self) {
        self.reg_write(REG_EOI, 0);
    }
}
