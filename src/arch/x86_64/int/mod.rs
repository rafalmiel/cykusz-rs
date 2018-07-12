mod apic;
mod pic;

pub trait InterruptController: Send + Sync {
    fn end_of_int(&self);
    fn irq_remap(&self, rq: u32) -> u32;
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
    static ref CONTROLLER : &'static InterruptController = &apic::CONTROLLER;
}

pub fn enable() {
    unsafe {
        asm!("sti");
    }
}

pub fn disable() {
    unsafe {
        asm!("cli");
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
