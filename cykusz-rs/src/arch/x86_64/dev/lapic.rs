use core::ptr::read_volatile;
use core::ptr::write_volatile;
use bit_field::BitField;
use crate::arch::acpi::apic::MadtHeader;
use crate::arch::idt;
use crate::arch::int;
use crate::arch::mm::MappedAddr;
use crate::arch::raw::msr;
use crate::kernel::ipi::IpiTarget;
use crate::kernel::sync::{IrqLock, LockApi};

pub static LAPIC: IrqLock<LApic> = IrqLock::new(LApic::new());

const REG_ID: u32 = 0x20;
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

const TIMER_MS: usize = 1;

pub struct LApic {
    lapic_base: Option<MappedAddr>,
    x2: bool,
    ticks_in_1_ms: u32,
}

impl LApic {
    pub const fn new() -> LApic {
        LApic {
            lapic_base: None,
            x2: false,
            ticks_in_1_ms: 0,
        }
    }

    fn reg_write(&self, reg: u32, value: u32) {
        if let Some(base) = self.lapic_base {
            unsafe {
                write_volatile::<u32>((base + reg as usize).0 as *mut u32, value);
            }
        } else {
            panic!("Failed write!");
        }
    }

    fn reg_read(&self, reg: u32) -> u32 {
        if let Some(base) = self.lapic_base {
            unsafe { read_volatile::<u32>((base + reg as usize).0 as *const u32) }
        } else {
            panic!("Failed read!");
        }
    }

    pub fn id(&self) -> u64 {
        if self.x2 {
            unsafe { msr::rdmsr(msr::IA32_X2APIC_APICID) }
        } else {
            (self.reg_read(REG_ID) >> 24) as u64
        }
    }

    pub fn init(&mut self, hdr: &'static MadtHeader) {
        self.x2 = crate::arch::cpu::has_x2apic();

        if !self.x2 {
            self.lapic_base = Some(hdr.lapic_address());

            // Clear task priority to enable all interrupts
            self.reg_write(REG_TRP, 0);

            // Logical Destination Mode
            self.reg_write(REG_DFR, 0xffffffff); // Flat mode
            self.reg_write(REG_LCR, 0x01000000); // All cpus use logical id 1

            // Configure Spurious Interrupt Vector Register
            self.reg_write(REG_SIVR, 0x100 | 0xff);
        } else {
            unsafe {
                //Enable X2APIC: (bit 10)
                msr::wrmsr(
                    msr::IA32_APIC_BASE,
                    msr::rdmsr(msr::IA32_APIC_BASE) | 1 << 10,
                );

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
                msr::wrmsr(
                    msr::IA32_APIC_BASE,
                    msr::rdmsr(msr::IA32_APIC_BASE) | 1 << 10,
                );

                // Clear task priority to enable all interrupts
                msr::wrmsr(msr::IA32_X2APIC_TPR, 0);

                // Configure Spurious Interrupt Vector Register
                msr::wrmsr(msr::IA32_X2APIC_SIVR, 0x100 | 0xff);
            }
        }
    }

    pub fn start_ap(&mut self, ap_id: u8) {
        use crate::arch::smp::AP_INIT;
        if !self.x2 {
            self.reg_write(REG_CMD_ID, (ap_id as u32) << 24);
            self.reg_write(REG_CMD, 0x4500);

            crate::kernel::timer::early_sleep(10);

            self.reg_write(REG_CMD_ID, (ap_id as u32) << 24);
            self.reg_write(REG_CMD, 0x4600u32 | ((AP_INIT.0 as u32) >> 12));

            crate::kernel::timer::early_sleep(10);
        } else {
            unsafe {
                // INIT
                msr::wrmsr(msr::IA32_X2APIC_ICR, 0x4500u64 | ((ap_id as u64) << 32));
                crate::kernel::timer::early_sleep(10);

                // START: AP INIT routine begins at physical address 0x1000
                msr::wrmsr(
                    msr::IA32_X2APIC_ICR,
                    0x4600u64 | ((AP_INIT.0 as u64) >> 12) | ((ap_id as u64) << 32),
                );
                crate::kernel::timer::early_sleep(10);
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

    pub fn send_ipi(&mut self, target: IpiTarget, vector: u8) {
        let (dest_type, dest) = target.get_dest_target();

        dbgln!(ipi, "is x2 {}", self.x2);

        if !self.x2 {
            self.reg_write(REG_CMD_ID, (dest as u32) << 24);

            let mut cmd = 0u32;
            cmd.set_bits(18..=19, dest_type as u32);
            cmd.set_bits(0..=7, vector as u32);

            self.reg_write(REG_CMD, cmd);

            let mut status = self.reg_read(REG_CMD) & (1u32 << 12);

            while status > 0 {
                status = self.reg_read(REG_CMD) & (1u32 << 12);
            }
        } else {
            unsafe {
                msr::wrmsr(
                    msr::IA32_X2APIC_ICR,
                    vector as u64 | ((dest_type as u64) << 18) | ((dest as u64) << 32),
                );

                let mut status = msr::rdmsr(msr::IA32_X2APIC_ICR) & (1u64 << 12);

                while status > 0 {
                    status = msr::rdmsr(msr::IA32_X2APIC_ICR) & (1u64 << 12);
                }
            }
        }
    }

    pub fn start_timer(&mut self, one_shot: bool) {
        if !self.x2 {
            self.reg_write(REG_TIMDIV, 0b11);
            self.reg_write(REG_TIM, 32 | (if one_shot { 0 } else { 1 } << 17));
            self.reg_write(REG_TIMINIT, self.ticks_in_1_ms as u32 * TIMER_MS as u32);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_DIV_CONF, 0b11);
                msr::wrmsr(
                    msr::IA32_X2APIC_LVT_TIMER,
                    32 | (if one_shot { 0 } else { 1 } << 17),
                );
                msr::wrmsr(
                    msr::IA32_X2APIC_INIT_COUNT,
                    self.ticks_in_1_ms as u64 * TIMER_MS as u64,
                );
            }
        }
    }

    pub fn reset_timer_counter(&mut self) {
        if !self.x2 {
            self.reg_write(REG_TIMINIT, self.ticks_in_1_ms as u32 * TIMER_MS as u32);
        } else {
            unsafe {
                msr::wrmsr(
                    msr::IA32_X2APIC_INIT_COUNT,
                    self.ticks_in_1_ms as u64 * TIMER_MS as u64,
                );
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

        crate::kernel::timer::early_sleep(ms);

        if !self.x2 {
            self.reg_write(REG_TIM, 1 << 16);
        } else {
            unsafe {
                msr::wrmsr(msr::IA32_X2APIC_LVT_TIMER, 1 << 16);
            }
        }

        let ticks = 0xFFFFFFFFu32
            - if !self.x2 {
                self.reg_read(REG_TIMCUR)
            } else {
                unsafe { msr::rdmsr(msr::IA32_X2APIC_CUR_COUNT) as u32 }
            };

        self.ticks_in_1_ms = ticks;

        //println!("[ INFO ] Ticks in {}ms: {}", ms, ticks);
    }
}

pub fn init(hdr: &'static MadtHeader) {
    LAPIC.irq().init(hdr);
}

pub fn init_ap() {
    LAPIC.irq().init_ap();
}

pub fn setup_timer(f: fn() -> bool) {
    int::set_irq_dest(0, 32);
    idt::add_shared_irq_handler(32, f);
    int::mask_int(0, false);
}

pub fn start_timer(one_shot: bool) {
    LAPIC.irq().start_timer(one_shot);
}

pub fn reset_timer_counter() {
    LAPIC.irq().reset_timer_counter();
}

pub fn start_ap() {
    use crate::arch::smp::Trampoline;

    let iter = {
        crate::arch::acpi::ACPI
            .lock()
            .get_apic_entry()
            .unwrap()
            .lapic_entries()
    };

    let bsp_id = LAPIC.irq().id();

    let mut cpu_id = 0;

    for cpu in iter {
        // Don't boot bootstrap processor
        if cpu.proc_is_enabled() && cpu.apic_id as u64 != bsp_id {
            let trampoline = Trampoline::get();

            trampoline.reset();

            cpu_id += 1;

            // Pass CPU ID to the new CPU
            trampoline.cpu_num = cpu_id;

            // Allocate stack for the new CPU
            trampoline.stack_ptr = unsafe {
                (crate::arch::mm::phys::allocate_order(4)
                    .unwrap()
                    .address_mapped()
                    .0 as *const u8)
                    .offset(4096 * 16)
            } as u64;

            // Pass page table pointer to the new CPU
            trampoline.page_table_ptr = unsafe { crate::arch::raw::ctrlregs::cr3() };

            {
                // Start AP and release the lock
                let mut lapic = LAPIC.irq();

                // Initialize new CPU
                lapic.start_ap(cpu.apic_id);
            }

            // Wait for the CPU to set the ready flag
            trampoline.wait_ready();
        }
    }
}
