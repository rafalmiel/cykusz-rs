use crate::arch::idt::ExceptionRegs;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::syscall::SyscallFrame;
use crate::kernel::sched::current_task;
use syscall_defs::{SyscallResult, SyscallError};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SyscallUserFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct SignalSigretutnFrame {
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
    let task = current_task();

    if task.signals().has_pending() {
        if let Some((sig, entry)) = task.signals().do_signals() {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let signal_frame = SignalSigretutnFrame {
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

                frame.sp -= core::mem::size_of::<SignalSigretutnFrame>() as u64;
                unsafe {
                    (frame.sp as *mut SignalSigretutnFrame).write(signal_frame);
                }

                frame.sp -= 8;
                unsafe {
                    (frame.sp as *mut usize).write(entry.sigreturn());
                }
                frame.ip = f as u64;

                println!("interrupt signal detected");

                // Signal param
                regs.rdi = sig as u64;

                crate::bochs();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arch_sys_check_signals(syscall_result: isize, sys_frame: &mut SyscallFrame) {
    let task = current_task();

    if task.signals().has_pending() {
        if let Some((sig, entry)) = task.signals().do_signals() {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let sys_user_frame = unsafe { *(sys_frame.rsp as *const SyscallUserFrame) };

                let res: SyscallResult = syscall_defs::SyscallFrom::syscall_from(syscall_result);

                let restart = if res == Err(SyscallError::Interrupted) {
                    entry.flags().contains(syscall_defs::signal::SignalFlags::RESTART)
                } else {
                    false
                };

                println!("sys_frame: {:?}", sys_frame);

                let signal_frame = SignalSigretutnFrame {
                    restart_syscall: if restart { sys_frame.rax } else { u64::MAX },
                    rax: if restart { sys_frame.rax } else { syscall_result as u64 },
                    rbx: sys_user_frame.rbx,
                    rcx: sys_frame.rcx,
                    rdx: sys_frame.rdx,
                    rsi: sys_frame.rsi,
                    rdi: sys_frame.rdi,
                    rbp: sys_user_frame.rbp,
                    r8: sys_frame.r8,
                    r9: sys_frame.r9,
                    r10: sys_frame.r10,
                    r11: sys_frame.r11,
                    r12: sys_user_frame.r12,
                    r13: sys_user_frame.r13,
                    r14: sys_user_frame.r14,
                    r15: sys_user_frame.r15,
                    rflags: sys_frame.r11,
                    rip: sys_frame.rcx,
                };
                println!("prepared signal frame: {:?}", signal_frame);

                sys_frame.rsp += core::mem::size_of::<SyscallUserFrame>() as u64;
                sys_frame.rsp -= core::mem::size_of::<SignalSigretutnFrame>() as u64;
                unsafe {
                    (sys_frame.rsp as *mut SignalSigretutnFrame).write(signal_frame);
                }

                sys_frame.rsp -= 8;
                unsafe {
                    (sys_frame.rsp as *mut usize).write(entry.sigreturn());
                }

                sys_frame.rsp -= core::mem::size_of::<SyscallUserFrame>() as u64;
                unsafe {
                    (sys_frame.rsp as *mut SyscallUserFrame).write(sys_user_frame);
                }

                sys_frame.rcx = f as u64;

                // Signal param
                sys_frame.rdi = sig as u64;

                crate::bochs();
            }
        }
    }
}

pub fn arch_sys_sigreturn(sys_frame: &mut SyscallFrame) -> isize {
    sys_frame.rsp += core::mem::size_of::<SyscallUserFrame>() as u64;

    let signal_frame = unsafe { (sys_frame.rsp as *const SignalSigretutnFrame).read() };

    let result = signal_frame.rax as isize;

    sys_frame.rdi = signal_frame.rdi;
    sys_frame.rsi = signal_frame.rsi;
    sys_frame.rdx = signal_frame.rdx;
    sys_frame.r10 = signal_frame.r10;
    sys_frame.r8 = signal_frame.r8;
    sys_frame.r9 = signal_frame.r9;

    sys_frame.r11 = signal_frame.rflags;
    sys_frame.rcx = signal_frame.rip;

    sys_frame.rsp += core::mem::size_of::<SignalSigretutnFrame>() as u64;
    sys_frame.rsp -= core::mem::size_of::<SyscallUserFrame>() as u64;

    let sys_user_frame = unsafe { &mut *(sys_frame.rsp as *mut SyscallUserFrame) };

    sys_user_frame.rbp = signal_frame.rbp;
    sys_user_frame.rbx = signal_frame.rbx;
    sys_user_frame.r12 = signal_frame.r12;
    sys_user_frame.r13 = signal_frame.r13;
    sys_user_frame.r14 = signal_frame.r14;
    sys_user_frame.r15 = signal_frame.r15;

    println!("returning sygnal_frame: {:?}", signal_frame);

    if signal_frame.restart_syscall != u64::MAX {
        println!("restarting, syscall num: {}", result);
        sys_frame.rcx -= 2;
    }

    crate::bochs();

    return result;
}
