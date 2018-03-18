use alloc::allocator::Alloc;
use core::ptr::write_volatile;
use core::ptr::read_volatile;

use arch::acpi::apic::MatdHeader;

use arch::mm::{MappedAddr,PhysAddr};
use arch::raw::msr;

use arch::sync::Mutex;

pub static LAPIC: Mutex<LApic> = Mutex::new(LApic::new());

const REG_TRP: u32 = 0x80;
const REG_LCR: u32 = 0xD0;
const REG_DFR: u32 = 0xE0;
const REG_SIVR: u32 = 0xF0;
const REG_EOI: u32 = 0xB0;

const REG_CMD: u32 = 0x300;
const REG_CMD_ID: u32 = 0x310;
const REG_TIM: u32 = 0x320;
const REG_TIMDIV: u32 = 0x3E0;
const REG_TIMINIT: u32 = 0x380;
const REG_TIMCUR: u32 = 0x390;

pub struct LApic {
    lapic_base: Option<MappedAddr>,
    x2: bool
}

impl LApic {
    pub const fn new() -> LApic {
        LApic {
            lapic_base: None,
            x2: false
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

    pub fn init(&mut self, hdr: &'static MatdHeader) {
        self.x2 = ::arch::dev::cpu::has_x2apic();

        if !self.x2 {
            self.lapic_base = Some(hdr.lapic_address());

            // Clear task priority to enable all interrupts
            self.reg_write(REG_TRP, 0);

            // Logical Destination Mode
            self.reg_write(REG_DFR, 0xffffffff);	// Flat mode
            self.reg_write(REG_LCR, 0x01000000);	// All cpus use logical id 1

            // Configure Spurious Interrupt Vector Register
            self.reg_write(REG_SIVR, 0x100 | 0xff);
        } else {
            unsafe {
                //Enable X2APIC: (bit 10)
                msr::wrmsr(msr::IA32_APIC_BASE, msr::rdmsr(msr::IA32_APIC_BASE) | 1 << 10);

                // Clear task priority to enable all interrupts
                msr::wrmsr(msr::IA32_X2APIC_TPR, 0);

                // Configure Spurious Interrupt Vector Register
                msr::wrmsr(msr::IA32_X2APIC_SIVR, 0x100 | 0xff);
            }
        }
    }

    pub fn init_ap(&mut self, ap_id: u8) {
        use arch::smp::{AP_INIT};
        if !self.x2 {
            self.reg_write(REG_CMD_ID, (ap_id as u32) << 24);
            self.reg_write(REG_CMD, 0x4500);

            ::arch::dev::pit::early_sleep(10);

            self.reg_write(REG_CMD_ID, (ap_id as u32) << 24);
            self.reg_write(REG_CMD, 0x4600u32 | ((AP_INIT.0 as u32) >> 12));

            ::arch::dev::pit::early_sleep(10);
        } else {
            unsafe {
                // INIT
                msr::wrmsr(msr::IA32_X2APIC_ICR, 0x4500u64 | ((ap_id as u64) << 32));
                ::arch::dev::pit::early_sleep(10);

                // START: AP INIT routine begins at physical address 0x1000
                msr::wrmsr(msr::IA32_X2APIC_ICR, 0x4600u64 | ((AP_INIT.0 as u64) >> 12) | ((ap_id as u64) << 32));
                ::arch::dev::pit::early_sleep(10);
            }
        }
    }

    pub fn end_of_int(&self) {
        if !self.x2 {
            self.reg_write(REG_EOI, 0);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_EOI, 0);
            }
        }
    }
}

pub fn init(hdr: &'static MatdHeader) {
    LAPIC.lock().init(hdr);
}

pub fn init_ap() {
    use arch::smp::{TRAMPOLINE};

    let mut lapic = LAPIC.lock_irq();

    for cpu in ::arch::acpi::ACPI.lock().get_rsdt().unwrap().find_apic_entry().unwrap().lapic_entries() {
        if cpu.proc_id > 0 {
            let rdy = TRAMPOLINE.to_mapped();
            unsafe {
                rdy.store::<u8>(0);
                (rdy + 1).store::<u8>(cpu.proc_id as u8);
                let sp = ::HEAP.alloc(::alloc::heap::Layout::from_size_align_unchecked(4096*16, 4096)).unwrap().offset(4096*16);
                (rdy + 2).store::<usize>(sp as usize);
                (rdy + 10).store::<usize>(::arch::raw::ctrlregs::cr3() as usize);
            }
            lapic.init_ap(cpu.proc_id);

            unsafe {
                while rdy.read_volatile::<u8>() == 0 {
                    asm!("pause"::::"volatile");
                }
            }

            println!("[ OK ] Initialized AP CPU: {}", cpu.proc_id);
        }
    }
}