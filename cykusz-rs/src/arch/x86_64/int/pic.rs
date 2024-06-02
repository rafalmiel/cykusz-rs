use crate::arch::dev::pic::PIC;
use crate::arch::int::InterruptController;
use crate::kernel::sync::LockApi;

pub struct Controller {}

#[allow(unused)]
pub static CONTROLLER: Controller = Controller {};

impl InterruptController for Controller {
    fn end_of_int(&self) {
        PIC.lock().notify_end_of_interrupt();
    }

    fn mask_int(&self, int: u8, masked: bool) {
        PIC.lock().mask_int(int, masked);
    }

    fn set_irq_dest(&self, _src: u8, _dst: u8) {}

    fn set_active_high(&self, _src: u8, _ah: bool) {
        unimplemented!()
    }

    fn set_level_triggered(&self, _src: u8, _ah: bool) {
        unimplemented!()
    }

    fn send_ipi(&self, _target_cpu: usize, _vector: usize) {
        // We won't have SMP with PIC controller anyway
        unimplemented!()
    }
}
