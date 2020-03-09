mod apic;
mod pic;

pub trait InterruptController: Send + Sync {
    fn end_of_int(&self);
    fn mask_int(&self, int: u8, masked: bool);
    fn set_irq_dest(&self, src: u8, dest: u8);
}

pub fn is_enabled() -> bool {
    unsafe {
        let r: usize;
        asm!("pushfq; popq $0" : "=r"(r) :: "memory");
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
        asm!("sti
        hlt"
        : : : : "intel", "volatile");
    }
}

/// Set interrupts and nop
/// This will enable interrupts and allow the IF flag to be processed
/// Simply enabling interrupts does not gurantee that they will trigger, use this instead!
#[inline(always)]
pub fn enable_and_nop() {
    unsafe {
        asm!("sti
        nop"
        : : : : "intel", "volatile");
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
