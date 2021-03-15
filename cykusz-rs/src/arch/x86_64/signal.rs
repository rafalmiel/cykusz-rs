use crate::arch::idt::ExceptionRegs;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::syscall::SyscallFrame;
use crate::kernel::sched::current_task;

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
pub struct SignalSigretutnFrame {
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
        if let Some(entry) = task.signals().do_signals() {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let signal_frame = SignalSigretutnFrame {
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

                crate::bochs();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arch_sys_check_signals(sys_frame: &mut SyscallFrame) {
    let task = current_task();

    if task.signals().has_pending() {
        if let Some(entry) = task.signals().do_signals() {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let sys_user_frame = unsafe { *(sys_frame.rsp as *const SyscallUserFrame) };

                let signal_frame = SignalSigretutnFrame {
                    rax: sys_frame.rax,
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

    crate::bochs();

    return result;
}
