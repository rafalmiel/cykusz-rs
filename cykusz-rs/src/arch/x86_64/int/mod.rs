use core::arch::asm;
use crate::kernel::ipi::IpiTarget;

mod apic;
pub mod msi;
mod pic;

pub trait InterruptController: Send + Sync {
    fn end_of_int(&self);
    fn mask_int(&self, int: u8, masked: bool);
    fn set_irq_dest(&self, src: u8, dest: u8);
    fn set_active_high(&self, src: u8, ah: bool);
    fn set_level_triggered(&self, src: u8, ah: bool);
    fn send_ipi(&self, target_cpu: IpiTarget, vector: usize);
}

pub fn is_enabled() -> bool {
    unsafe {
        let r: usize;
        asm!("pushfq", "pop {r}", r = out(reg) r);
        return (r & (1usize << 9)) > 0;
    }
}

lazy_static! {
    static ref CONTROLLER: &'static dyn InterruptController = &apic::CONTROLLER;
}

pub fn enable() {
    enable_and_nop();
}

pub fn disable() {
    unsafe {
        asm!("cli");
    }
}

/// Set interrupts and halt
/// This will atomically wait for the next interrupt
/// Performing enable followed by halt is not guaranteed to be atomic, use this instead!
#[inline(always)]
pub fn enable_and_halt() {
    unsafe {
        asm!("sti", "hlt");
    }
}

/// Set interrupts and nop
/// This will enable interrupts and allow the IF flag to be processed
/// Simply enabling interrupts does not gurantee that they will trigger, use this instead!
#[inline(always)]
pub fn enable_and_nop() {
    unsafe {
        asm!("sti", "nop");
    }
}

pub fn end_of_int() {
    CONTROLLER.end_of_int();
}

pub fn mask_int(int: u8, masked: bool) {
    CONTROLLER.mask_int(int, masked);
}

pub fn set_irq_dest(src: u8, dst: u8) {
    CONTROLLER.set_irq_dest(src, dst);
}

pub fn set_active_high(src: u8, ah: bool) {
    CONTROLLER.set_active_high(src, ah)
}

pub fn set_level_triggered(src: u8, ah: bool) {
    CONTROLLER.set_level_triggered(src, ah)
}

pub fn send_ipi(target_cpu: IpiTarget, vector: u8) {
    CONTROLLER.send_ipi(target_cpu, vector as usize)
}
