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

#[repr(C)]
#[derive(Debug)]
pub struct SyscallFrame {
    pub rax: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub r10: u64,
    pub r8: u64,
    pub r9: u64,

    pub r11: u64, // rflags
    pub rcx: u64, // rip
    pub rsp: u64, // rsp
}

#[no_mangle]
pub extern "C" fn fast_syscall_handler(frame: &mut SyscallFrame) -> isize {
    if frame.rax == syscall_defs::SYS_SIGRETURN as u64 {
        crate::arch::signal::arch_sys_sigreturn(frame)
    } else {
        let res = crate::kernel::syscall::syscall_handler(
            frame.rax, frame.rdi, frame.rsi, frame.rdx, frame.r10, frame.r8, frame.r9,
        );

        crate::arch::signal::arch_sys_check_signals(res, frame);

        res
    }
}
