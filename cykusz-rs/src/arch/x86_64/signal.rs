use syscall_defs::{SyscallError, SyscallResult};

use crate::arch::idt::ExceptionRegs;
use crate::arch::raw::idt::InterruptFrame;
use crate::arch::syscall::SyscallFrame;

const SYSCALL_INSTRUCTION_SIZE: u64 = 2;

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

impl SigReturnFrame {
    fn from_interrupt(frame: &mut InterruptFrame, regs: &mut ExceptionRegs) -> SigReturnFrame {
        SigReturnFrame {
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
        }
    }

    fn from_syscall(
        restart: bool,
        syscall_result: u64,
        sys_frame: &mut SyscallFrame,
        regs: &mut ExceptionRegs,
    ) -> SigReturnFrame {
        SigReturnFrame {
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
        }
    }
}

struct StackWriter<'a> {
    ptr: &'a mut u64,
}

impl<'a> StackWriter<'a> {
    fn new(ptr: &'a mut u64) -> StackWriter<'a> {
        StackWriter::<'a> { ptr }
    }

    fn skip(&mut self, by: u64) {
        *self.ptr -= by;
    }

    fn skip_redzone(&mut self) {
        self.skip(128);
    }

    fn write<T: Sized>(&mut self, val: T) {
        *self.ptr -= core::mem::size_of::<T>() as u64;

        unsafe {
            (*self.ptr as *mut T).write(val);
        }
    }

    fn restore(&mut self, by: u64) {
        *self.ptr += by;
    }

    fn restore_redzone(&mut self) {
        self.restore(128);
    }

    fn read<T: Sized>(&mut self) -> T {
        let v = unsafe {
            (*self.ptr as *mut T).read()
        };

        *self.ptr += core::mem::size_of::<T>() as u64;

        v
    }
}

impl ExceptionRegs {
    fn load_signal_frame(&mut self, frame: &SigReturnFrame) {
        self.rdi = frame.rdi;
        self.rsi = frame.rsi;
        self.rdx = frame.rdx;
        self.r10 = frame.r10;
        self.r8 = frame.r8;
        self.r9 = frame.r9;

        self.r11 = frame.r11;
        self.rcx = frame.rcx;

        self.rbp = frame.rbp;
        self.rbx = frame.rbx;
        self.r12 = frame.r12;
        self.r13 = frame.r13;
        self.r14 = frame.r14;
        self.r15 = frame.r15;
    }
}

pub fn arch_int_check_signals(frame: &mut InterruptFrame, regs: &mut ExceptionRegs) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let signal_frame = SigReturnFrame::from_interrupt(frame, regs);

            let mut writer = StackWriter::new(&mut frame.sp);

            writer.skip_redzone();
            writer.write(signal_frame);
            writer.write(entry.sigreturn());

            frame.ip = f as u64;

            // Signal param
            regs.rdi = sig as u64;
        }
    }
}

#[no_mangle]
pub extern "C" fn arch_sys_check_signals(
    syscall_result: isize,
    sys_frame: &mut SyscallFrame,
    regs: &mut ExceptionRegs,
) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let res: SyscallResult = syscall_defs::SyscallFrom::syscall_from(syscall_result);

            let restart = res == Err(SyscallError::Interrupted)
                && entry
                    .flags()
                    .contains(syscall_defs::signal::SignalFlags::RESTART);

            let signal_frame =
                SigReturnFrame::from_syscall(restart, syscall_result as u64, sys_frame, regs);

            let mut writer = StackWriter::new(&mut sys_frame.rsp);

            writer.skip_redzone();
            writer.write(signal_frame);
            writer.write(entry.sigreturn());

            sys_frame.rip = f as u64;

            // Signal param
            regs.rdi = sig as u64;
        }
    }
}

pub fn arch_sys_sigreturn(sys_frame: &mut SyscallFrame, user_regs: &mut ExceptionRegs) -> isize {
    let mut writer = StackWriter::new(&mut sys_frame.rsp);

    let signal_frame = writer.read::<SigReturnFrame>();
    writer.restore_redzone();

    let result = signal_frame.rax as isize;

    user_regs.load_signal_frame(&signal_frame);

    sys_frame.rflags = signal_frame.rflags;
    sys_frame.rip = signal_frame.rip;

    if signal_frame.restart_syscall != u64::MAX {
        sys_frame.rip -= SYSCALL_INSTRUCTION_SIZE;
    }

    crate::bochs();

    return result;
}
