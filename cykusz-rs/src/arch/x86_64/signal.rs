use syscall_defs::{SyscallError, SyscallResult};

use crate::arch::idt::RegsFrame;
use crate::arch::raw::idt::InterruptFrame;
use crate::arch::syscall::SyscallFrame;
use crate::arch::utils::StackHelper;

const SYSCALL_INSTRUCTION_SIZE: u64 = 2;
const REDZONE_SIZE: u64 = 128;

#[repr(C)]
#[derive(Debug)]
pub struct SignalFrame {
    restart_syscall: u64,
    regs: RegsFrame,
    rflags: u64,
    rip: u64,
}

impl SignalFrame {
    fn from_interrupt(frame: &mut InterruptFrame, regs: &mut RegsFrame) -> SignalFrame {
        SignalFrame {
            restart_syscall: u64::MAX,
            regs: *regs,
            rflags: frame.cf,
            rip: frame.ip,
        }
    }

    fn from_syscall(
        restart: bool,
        syscall_result: u64,
        sys_frame: &mut SyscallFrame,
        regs: &mut RegsFrame,
    ) -> SignalFrame {
        let mut frame = SignalFrame {
            restart_syscall: if restart { regs.rax } else { u64::MAX },
            regs: *regs,
            rflags: sys_frame.rflags,
            rip: sys_frame.rip,
        };

        if !restart {
            frame.regs.rax = syscall_result;
        }

        frame
    }
}

pub fn arch_int_check_signals(frame: &mut InterruptFrame, regs: &mut RegsFrame) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let signal_frame = SignalFrame::from_interrupt(frame, regs);

            let mut writer = StackHelper::new(&mut frame.sp);

            writer.skip_by(REDZONE_SIZE);
            unsafe {
                writer.write(signal_frame);
                writer.write(entry.sigreturn());
            }

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
    regs: &mut RegsFrame,
) {
    if let Some((sig, entry)) = crate::kernel::signal::do_signals() {
        if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
            let res: SyscallResult = syscall_defs::SyscallFrom::syscall_from(syscall_result);

            let restart = res == Err(SyscallError::EINTR)
                && entry
                    .flags()
                    .contains(syscall_defs::signal::SignalFlags::RESTART);

            let signal_frame =
                SignalFrame::from_syscall(restart, syscall_result as u64, sys_frame, regs);

            let mut writer = StackHelper::new(&mut sys_frame.rsp);

            writer.skip_by(REDZONE_SIZE);
            unsafe {
                writer.write(signal_frame);
                writer.write(entry.sigreturn());
            }

            sys_frame.rip = f as u64;

            // Signal param
            regs.rdi = sig as u64;
        }
    }
}

pub fn arch_sys_sigreturn(sys_frame: &mut SyscallFrame, user_regs: &mut RegsFrame) -> isize {
    let mut writer = StackHelper::new(&mut sys_frame.rsp);

    let signal_frame = unsafe { writer.restore::<SignalFrame>() };

    writer.restore_by(REDZONE_SIZE);

    let result = signal_frame.regs.rax as isize;

    *user_regs = signal_frame.regs;

    sys_frame.rflags = signal_frame.rflags;
    sys_frame.rip = signal_frame.rip;

    if signal_frame.restart_syscall != u64::MAX {
        sys_frame.rip -= SYSCALL_INSTRUCTION_SIZE as u64;
    }

    return result;
}
