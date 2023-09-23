use syscall_defs::{SyscallFrom, SyscallInto, SyscallRestartable, SyscallResult};

use crate::arch::idt::RegsFrame;
use crate::arch::raw::idt::InterruptFrame;
use crate::arch::syscall::SyscallFrame;
use crate::arch::utils::StackHelper;
use crate::kernel::sched::current_task_ref;
use crate::kernel::signal::DoSignalsResult;

const SYSCALL_INSTRUCTION_SIZE: u64 = 2;
const REDZONE_SIZE: u64 = 128;

#[repr(C)]
#[derive(Debug)]
pub struct SignalFrame {
    restart_syscall: u64,
    regs: RegsFrame,
    sigmask: u64,
    rflags: u64,
    rip: u64,
}

impl SignalFrame {
    fn from_interrupt(
        frame: &mut InterruptFrame,
        regs: &mut RegsFrame,
        sigmask: u64,
    ) -> SignalFrame {
        SignalFrame {
            restart_syscall: u64::MAX,
            regs: *regs,
            sigmask,
            rflags: frame.cf,
            rip: frame.ip,
        }
    }

    fn from_syscall(
        restart: bool,
        syscall_result: u64,
        sys_frame: &mut SyscallFrame,
        regs: &mut RegsFrame,
        sigmask: u64,
    ) -> SignalFrame {
        let mut frame = SignalFrame {
            restart_syscall: if restart { regs.rax } else { u64::MAX },
            regs: *regs,
            sigmask,
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
    match crate::kernel::signal::do_signals(false, 0) {
        DoSignalsResult::Entry((sig, entry)) => {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let signals = current_task_ref().signals();
                let old_mask = signals.blocked_mask();

                let signal_frame = SignalFrame::from_interrupt(frame, regs, old_mask);

                signals.set_mask(
                    syscall_defs::signal::SigProcMask::Block,
                    Some(1u64 << (sig - 1)),
                    None,
                );

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
        DoSignalsResult::Default | DoSignalsResult::Ignore => {
            arch_int_check_signals(frame, regs);
        }
        _ => {}
    }
}

pub fn arch_sys_check_signals(
    syscall_result: SyscallResult,
    sys_frame: &mut SyscallFrame,
    regs: &mut RegsFrame,
) -> bool {
    match crate::kernel::signal::do_signals(true, regs.rax as usize) {
        DoSignalsResult::Entry((sig, entry)) => {
            if let syscall_defs::signal::SignalHandler::Handle(f) = entry.handler() {
                let signals = current_task_ref().signals();
                let old_mask = signals.blocked_mask();

                signals.set_mask(
                    syscall_defs::signal::SigProcMask::Block,
                    Some(1u64 << (sig - 1)),
                    None,
                );

                let restart = syscall_result.is_restart(
                    true,
                    entry
                        .flags()
                        .contains(syscall_defs::signal::SignalFlags::RESTART),
                );

                let signal_frame = SignalFrame::from_syscall(
                    restart,
                    syscall_result.syscall_into() as u64,
                    sys_frame,
                    regs,
                    old_mask,
                );

                let mut writer = StackHelper::new(&mut sys_frame.rsp);

                writer.skip_by(REDZONE_SIZE);
                unsafe {
                    writer.write(signal_frame);
                    writer.write(entry.sigreturn());
                }

                logln_disabled!("sys_frame set rip: {:#x}", f as usize);
                sys_frame.rip = f as u64;

                // Signal param
                regs.rdi = sig as u64;

                logln2!("do signal!!! {} FROM SYSCALL", sig);
            }
            return false;
        }
        DoSignalsResult::Default | DoSignalsResult::Ignore => {
            if syscall_result.is_restart(false, false) {
                sys_frame.rip -= SYSCALL_INSTRUCTION_SIZE;
                return true;
            } else {
                arch_sys_check_signals(syscall_result, sys_frame, regs);
                return false;
            }
        }
        _ => {
            // Store syscall result in rax
            regs.rax = syscall_result.syscall_into() as u64;
            return false;
        }
    }
}

pub fn arch_sys_sigreturn(
    sys_frame: &mut SyscallFrame,
    user_regs: &mut RegsFrame,
) -> (SyscallResult, bool) {
    let mut writer = StackHelper::new(&mut sys_frame.rsp);

    let signal_frame = unsafe { writer.restore::<SignalFrame>() };

    current_task_ref().signals().set_mask(
        syscall_defs::signal::SigProcMask::Set,
        Some(signal_frame.sigmask),
        None,
    );

    writer.restore_by(REDZONE_SIZE);

    let result = signal_frame.regs.rax as isize;

    *user_regs = signal_frame.regs;

    sys_frame.rflags = signal_frame.rflags;
    sys_frame.rip = signal_frame.rip;

    let was_restart = if signal_frame.restart_syscall != u64::MAX {
        sys_frame.rip -= SYSCALL_INSTRUCTION_SIZE as u64;
        true
    } else {
        false
    };

    return (SyscallResult::syscall_from(result), was_restart);
}
