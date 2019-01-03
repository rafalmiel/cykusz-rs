use ::arch::dev::pic::PIC;
use ::arch::int::InterruptController;

pub struct Controller {}

#[allow(unused)]
pub static CONTROLLER: Controller = Controller {};

impl InterruptController for Controller {
    fn end_of_int(&self) {
        PIC.lock().notify_end_of_interrupt();
    }

    fn irq_remap(&self, irq: u32) -> u32 {
        return irq;
    }

    fn mask_int(&self, int: u8, masked: bool) {
        PIC.lock().mask_int(int, masked);
    }

    fn set_irq_dest(&self, _src: u8, _dst: u8) {
    }
}

