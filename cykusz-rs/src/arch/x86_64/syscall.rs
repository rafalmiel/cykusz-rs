use crate::arch::idt::RegsFrame;

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
#[derive(Copy, Clone, Debug)]
pub struct SyscallFrame {
    pub rflags: u64, // rflags
    pub rip: u64,    // rip
    pub rsp: u64,    // rsp
}

#[no_mangle]
pub extern "C" fn fast_syscall_handler(
    sys_frame: &mut SyscallFrame,
    regs: &mut RegsFrame,
) {
    if regs.rax == syscall_defs::SYS_SIGRETURN as u64 {

        // Store syscall result in rax
        regs.rax = crate::arch::signal::arch_sys_sigreturn(sys_frame, regs) as u64
    } else {
        let res = crate::kernel::syscall::syscall_handler(
            regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9,
        );

        crate::arch::signal::arch_sys_check_signals(res, sys_frame, regs);

        // Store syscall result in rax
        regs.rax = res as u64
    }
}
