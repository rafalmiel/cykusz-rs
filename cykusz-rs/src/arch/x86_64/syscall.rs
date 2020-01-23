use crate::arch::idt;
use crate::arch::raw::idt as ridt;

extern "C" {
    fn asm_syscall_handler();
}

fn enable_syscall_extension() {
    use crate::arch::raw::msr;
    unsafe {
        msr::wrmsr(msr::IA32_EFER, msr::rdmsr(msr::IA32_EFER) | 1);

        msr::wrmsr(msr::IA32_STAR, 0x0013_0008_0000_0000);
        msr::wrmsr(msr::IA32_LSTAR, asm_syscall_handler as u64);
        msr::wrmsr(msr::IA32_FMASK, 0x200);
        msr::wrmsr(
            msr::IA32_KERNEL_GS_BASE,
            &crate::arch::gdt::TSS as *const _ as u64,
        );
    }
}

pub fn init() {
    enable_syscall_extension();

    idt::set_user_handler(80, syscall_handler);
}

pub fn init_ap() {
    enable_syscall_extension();
}

#[repr(C, packed)]
pub struct SyscallFrame {
    rax: u64,
    rdi: u64,
    rsi: u64,
}

#[no_mangle]
pub extern "C" fn fast_syscall_handler(frame: &SyscallFrame) -> u64 {
    crate::kernel::syscall::syscall_handler(frame.rax, frame.rdi, frame.rsi)
}

extern "x86-interrupt" fn syscall_handler(_frame: &mut ridt::ExceptionStackFrame) {
    crate::kernel::syscall::syscall_handler(0, 0, 0);
}
