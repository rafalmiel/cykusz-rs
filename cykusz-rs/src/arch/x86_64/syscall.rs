use syscall_defs::{SyscallError, SyscallInto, SyscallResult, SYSCALL_STRING};

use crate::arch::idt::RegsFrame;
use crate::arch::signal::arch_sys_check_signals;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::IrqGuard;

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
pub extern "C" fn fast_syscall_handler(sys_frame: &mut SyscallFrame, regs: &mut RegsFrame) {
    if regs.rax == syscall_defs::SYS_SIGRETURN as u64 {
        // Store syscall result in rax
        let (res, was_restart) = crate::arch::signal::arch_sys_sigreturn(sys_frame, regs);

        if !was_restart {
            arch_sys_check_signals(res, sys_frame, regs);
        } else {
            regs.rax = res.syscall_into() as u64;
        }
    } else {
        //logln!("syscall {:?} {:?}", regs, sys_frame);
        let task = current_task_ref();
        dbgln!(
            syscall,
            "syscall [task {}] {} {}, ret: 0x{:x}",
            task.tid(),
            regs.rax,
            SYSCALL_STRING[regs.rax as usize],
            sys_frame.rip
        );

        let res = crate::kernel::syscall::syscall_handler(
            regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9,
        );

        dbgln!(
            syscall,
            "done syscall [task {}] {} = {:?}",
            task.tid(),
            SYSCALL_STRING[regs.rax as usize],
            res
        );

        crate::arch::signal::arch_sys_check_signals(res, sys_frame, regs);
    }
}

pub fn sys_arch_prctl(cmd: u64, addr: u64) -> SyscallResult {
    use syscall_defs::prctl::PrctlCmd;

    let cmd: PrctlCmd = (cmd as usize).into();

    match cmd {
        PrctlCmd::ArchSetFs => {
            //println!("[ ARCH ] Set FS: {:#x}", addr);
            let addr = VirtAddr(addr as usize);

            if !addr.is_user() {
                return Err(SyscallError::EINVAL);
            }

            let task = current_task_ref();

            unsafe {
                let _guard = IrqGuard::new();
                //print!("!");
                task.arch_task_mut().update_user_fs(addr);
            }

            Ok(0)
        }
        PrctlCmd::Unknown => Err(SyscallError::EINVAL),
    }
}
