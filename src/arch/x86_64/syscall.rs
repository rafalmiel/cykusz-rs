use arch::idt;
use arch::raw::idt as ridt;

pub fn init() {
    idt::set_user_handler(80, syscall_handler);
}

extern "x86-interrupt" fn syscall_handler(_frame: &mut ridt::ExceptionStackFrame) {
    ::kernel::syscall::syscall_handler(0);
}
