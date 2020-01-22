use crate::kernel::mm::VirtAddr;

pub fn init(stack_top: VirtAddr) {
    crate::arch::tls::init(stack_top);
}

pub fn is_ready() -> bool {
    crate::kernel::smp::is_smp_initialised()
}
