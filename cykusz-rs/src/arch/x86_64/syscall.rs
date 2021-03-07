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
    }
}

pub fn init() {
    enable_syscall_extension();
}

pub fn init_ap() {
    enable_syscall_extension();
}

#[repr(C, packed)]
pub struct SyscallFrame {
    rax: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    r10: u64,
    r8: u64,
    r9: u64,
}

#[no_mangle]
pub extern "C" fn fast_syscall_handler(frame: &SyscallFrame) -> isize {
    crate::kernel::syscall::syscall_handler(
        frame.rax, frame.rdi, frame.rsi, frame.rdx, frame.r10, frame.r8, frame.r9,
    )
}
