use alloc::allocator::Alloc;
use core::ptr::write_volatile;
use core::ptr::read_volatile;

use arch::acpi::apic::MatdHeader;

use arch::mm::{MappedAddr};
use arch::raw::msr;

use arch::sync::Mutex;
use arch::int;
use arch::idt;
use arch::raw::idt as ridt;

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
    x2: bool,
    ticks_in_1_ms: u32
}

impl LApic {
    pub const fn new() -> LApic {
        LApic {
            lapic_base: None,
            x2: false,
            ticks_in_1_ms: 0
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
        self.ticks_in_ms(1);
        self.ticks_in_ms(1);
        self.ticks_in_ms(1);
        self.ticks_in_ms(1);
    }

    pub fn init_ap(&mut self) {
        if !self.x2 {
            // Clear task priority to enable all interrupts
            self.reg_write(REG_TRP, 0);

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

    pub fn start_ap(&mut self, ap_id: u8) {
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

    pub fn start_timer(&mut self) {
        if !self.x2 {
            self.reg_write(REG_TIMDIV, 0b11);
            self.reg_write(REG_TIM, 32 | (1<<17));
            self.reg_write(REG_TIMINIT, self.ticks_in_1_ms as u32 * 1000);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_DIV_CONF, 0b11);
                msr::wrmsr(msr::IA32_X2APIC_LVT_TIMER, 32 | (1<<17));
                msr::wrmsr(msr::IA32_X2APIC_INIT_COUNT, self.ticks_in_1_ms as u64 * 1000);
            }
        }
    }

    pub fn ticks_in_ms(&mut self, ms: u64) {
        if !self.x2 {
            self.reg_write(REG_TIMDIV, 0b11);
            self.reg_write(REG_TIMINIT, 0xFFFFFFFF);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_DIV_CONF, 0b11);
                msr::wrmsr(msr::IA32_X2APIC_INIT_COUNT, 0xFFFFFFFF);
            }
        }

        ::arch::dev::pit::early_sleep(ms);

        if !self.x2 {
            self.reg_write(REG_TIM, 1<<16);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_LVT_TIMER, 1<<16);
            }
        }

        let ticks = 0xFFFFFFFFu32 -
            if !self.x2 {
                self.reg_read(REG_TIMCUR)
            } else {
                unsafe {
                    msr::rdmsr(msr::IA32_X2APIC_CUR_COUNT) as u32
                }
            };

        self.ticks_in_1_ms = ticks;

        println!("[ INFO ] Ticks in {}ms: {}", ms, ticks);
    }
}

pub fn init(hdr: &'static MatdHeader) {
    LAPIC.lock().init(hdr);
}

pub fn init_ap() {
    LAPIC.lock().init_ap();
}

pub fn start_timer() {
    let remap: u8 = int::get_irq_mapping(0) as u8;
    int::set_irq_dest(remap as u8, 32);
    idt::set_handler(32, lapic_timer_handler);

    int::mask_int(remap, false);

    LAPIC.lock().start_timer();
}

pub extern "x86-interrupt" fn lapic_timer_handler(_frame: &mut ridt::ExceptionStackFrame) {
    unsafe {
        print!("{}", ::CPU_ID);
    }
    int::end_of_int();
}

pub fn start_ap() {
    use arch::smp::{Trampoline};

    let iter = {
        ::arch::acpi::ACPI.lock().get_apic_entry().unwrap().lapic_entries()
    };


    for cpu in iter {

        // Don't boot bootstrap processor
        if cpu.proc_id > 0 {

            let trampoline = Trampoline::get();
            trampoline.reset();

            // Pass CPU ID to the new CPU
            trampoline.cpu_num = cpu.proc_id as u8;

            // Allocate stack for the new CPU
            trampoline.stack_ptr = unsafe {
                ::HEAP.alloc(
                    ::alloc::heap::Layout::from_size_align_unchecked(4096 * 16, 4096)
                ).unwrap().offset(4096 * 16)
            } as u64;

            // Pass page table pointer to the new CPU
            trampoline.page_table_ptr = unsafe {
                ::arch::raw::ctrlregs::cr3()
            };

            {
                // Start AP and release the lock
                let mut lapic = LAPIC.lock_irq();

                // Initialize new CPU
                lapic.start_ap(cpu.proc_id);
            }

            // Wait for the CPU to set the ready flag
            trampoline.wait_ready();
        }
    }
}