use syscall_defs::{SyscallError, SyscallResult};

use crate::arch::idt::ExceptionRegs;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::syscall::SyscallFrame;

#[repr(C)]
#[derive(Debug)]
pub struct SigReturnFrame {
    restart_syscall: u64,
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rflags: u64,
    rip: u64,
}

pub fn arch_int_check_signals(frame: &mut ExceptionStackFrame, regs: &mut ExceptionRegs) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let signal_frame = SigReturnFrame {
                restart_syscall: u64::MAX,
                rax: regs.rax,
                rbx: regs.rbx,
                rcx: regs.rcx,
                rdx: regs.rdx,
                rsi: regs.rsi,
                rdi: regs.rdi,
                rbp: regs.rbp,
                r8: regs.r8,
                r9: regs.r9,
                r10: regs.r10,
                r11: regs.r11,
                r12: regs.r12,
                r13: regs.r13,
                r14: regs.r14,
                r15: regs.r15,
                rflags: frame.cf,
                rip: frame.ip,
            };

            frame.sp -= 128; // Don't override red zone

            frame.sp -= core::mem::size_of::<SigReturnFrame>() as u64;
            unsafe {
                (frame.sp as *mut SigReturnFrame).write(signal_frame);
            }

            frame.sp -= 8;
            unsafe {
                (frame.sp as *mut usize).write(entry.sigreturn());
            }
            frame.ip = f as u64;

            // Signal param
            regs.rdi = sig as u64;
        }
    }
}

#[no_mangle]
pub extern "C" fn arch_sys_check_signals(syscall_result: isize, regs: &mut ExceptionRegs, sys_frame: &mut SyscallFrame) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let res: SyscallResult = syscall_defs::SyscallFrom::syscall_from(syscall_result);

            let restart = res == Err(SyscallError::Interrupted)
                && entry
                    .flags()
                    .contains(syscall_defs::signal::SignalFlags::RESTART);

            let signal_frame = SigReturnFrame {
                restart_syscall: if restart { regs.rax } else { u64::MAX },
                rax: if restart {
                    regs.rax
                } else {
                    syscall_result as u64
                },
                rbx: regs.rbx,
                rcx: regs.rcx,
                rdx: regs.rdx,
                rsi: regs.rsi,
                rdi: regs.rdi,
                rbp: regs.rbp,
                r8: regs.r8,
                r9: regs.r9,
                r10: regs.r10,
                r11: regs.r11,
                r12: regs.r12,
                r13: regs.r13,
                r14: regs.r14,
                r15: regs.r15,
                rflags: sys_frame.rflags,
                rip: sys_frame.rip,
            };

            sys_frame.rsp -= 128; // Don't override red zone

            sys_frame.rsp -= core::mem::size_of::<SigReturnFrame>() as u64;
            unsafe {
                (sys_frame.rsp as *mut SigReturnFrame).write(signal_frame);
            }

            sys_frame.rsp -= 8;
            unsafe {
                (sys_frame.rsp as *mut usize).write(entry.sigreturn());
            }

            sys_frame.rip = f as u64;

            // Signal param
            regs.rdi = sig as u64;
        }
    }
}

pub fn arch_sys_sigreturn(user_regs: &mut ExceptionRegs, sys_frame: &mut SyscallFrame) -> isize {
    let signal_frame = unsafe { (sys_frame.rsp as *const SigReturnFrame).read() };

    let result = signal_frame.rax as isize;

    user_regs.rdi = signal_frame.rdi;
    user_regs.rsi = signal_frame.rsi;
    user_regs.rdx = signal_frame.rdx;
    user_regs.r10 = signal_frame.r10;
    user_regs.r8 = signal_frame.r8;
    user_regs.r9 = signal_frame.r9;

    user_regs.r11 = signal_frame.r11;
    user_regs.rcx = signal_frame.rcx;

    user_regs.rbp = signal_frame.rbp;
    user_regs.rbx = signal_frame.rbx;
    user_regs.r12 = signal_frame.r12;
    user_regs.r13 = signal_frame.r13;
    user_regs.r14 = signal_frame.r14;
    user_regs.r15 = signal_frame.r15;

    sys_frame.rsp += core::mem::size_of::<SigReturnFrame>() as u64;
    sys_frame.rsp += 128; // Restore red zone

    sys_frame.rflags = signal_frame.rflags;
    sys_frame.rip = signal_frame.rip;

    if signal_frame.restart_syscall != u64::MAX {
        sys_frame.rip -= 2;
    }

    crate::bochs();

    return result;
}
