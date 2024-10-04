use alloc::vec::Vec;

use crate::arch::raw::idt;
use crate::arch::raw::idt::InterruptFrame;
use crate::arch::tls::restore_user_fs;
use crate::arch::x86_64::int::end_of_int;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::{current_task, current_task_ref};
use crate::kernel::sync::Spin;
use crate::kernel::sync::{LockApi, RwSpin};
use crate::kernel::task::vm::PageFaultReason;

static IDT: Spin<idt::Idt> = Spin::new(idt::Idt::new());

pub type ExceptionFn = fn(&mut InterruptFrame, &mut RegsFrame);
pub type ExceptionErrFn = fn(&mut InterruptFrame, &mut RegsFrame, u64);
pub type InterruptFn = fn();
pub type SharedInterruptFn = fn() -> bool;

enum IrqHandler {
    Missing,
    Exception(ExceptionFn),
    ExceptionErr(ExceptionErrFn),
    Interrupt(InterruptFn),
    SharedInterrupt(Vec<SharedInterruptFn>),
}

struct Irqs {
    irqs: [RwSpin<IrqHandler>; 256],
}

impl Irqs {
    fn set_exception_handler(&self, idx: usize, f: ExceptionFn) {
        let mut irqs = self.irqs[idx].write_irq();

        match *irqs {
            IrqHandler::Missing => {
                *irqs = IrqHandler::Exception(f);
            }
            _ => {
                panic!("Exception handler already exists");
            }
        }
    }

    fn set_exception_err_handler(&self, idx: usize, f: ExceptionErrFn) {
        let mut irqs = self.irqs[idx].write_irq();

        match *irqs {
            IrqHandler::Missing => {
                *irqs = IrqHandler::ExceptionErr(f);
            }
            _ => {
                panic!("ExceptionErr handler already exists");
            }
        }
    }

    fn set_int_handler(&self, idx: usize, f: InterruptFn) {
        let mut irqs = self.irqs[idx].write_irq();

        match *irqs {
            IrqHandler::Missing => {
                *irqs = IrqHandler::Interrupt(f);
            }
            _ => {
                panic!("Interrupt handler already exists");
            }
        }
    }

    fn alloc_int_handler(&self, f: InterruptFn) -> Option<usize> {
        for i in 64..256 {
            let mut irq = self.irqs[i].write_irq();

            if let IrqHandler::Missing = *irq {
                *irq = IrqHandler::Interrupt(f);

                return Some(i);
            }
        }

        None
    }

    fn set_shared_int_handler(&self, idx: usize, f: SharedInterruptFn) {
        let mut irqs = self.irqs[idx].write_irq();

        match &mut *irqs {
            IrqHandler::Missing => {
                let mut v = Vec::<SharedInterruptFn>::new();
                v.push(f);
                *irqs = IrqHandler::SharedInterrupt(v);
            }
            IrqHandler::SharedInterrupt(v) => {
                v.push(f);
            }
            _ => {
                panic!("Not a shared interrupt handler");
            }
        }
    }

    fn remove_shared_int_handler(&self, idx: usize, handler: SharedInterruptFn) {
        let mut irqs = self.irqs[idx].write_irq();

        if let IrqHandler::SharedInterrupt(h) = &mut *irqs {
            if let Some(i) = h.iter().enumerate().find_map(|(i, e)| {
                if *e == handler {
                    return Some(i);
                } else {
                    None
                }
            }) {
                h.remove(i);
            }
        }
    }

    pub fn set_divide_by_zero(&self, f: ExceptionFn) {
        self.set_exception_handler(0, f);
    }
    pub fn set_debug(&self, f: ExceptionFn) {
        self.set_exception_handler(1, f);
    }
    pub fn set_non_maskable_interrupt(&self, f: ExceptionFn) {
        self.set_exception_handler(2, f);
    }
    pub fn set_breakpoint(&self, f: ExceptionFn) {
        self.set_exception_handler(3, f);
    }
    pub fn set_overflow(&self, f: ExceptionFn) {
        self.set_exception_handler(4, f);
    }
    pub fn set_bound_range_exceeded(&self, f: ExceptionFn) {
        self.set_exception_handler(5, f);
    }
    pub fn set_invalid_opcode(&self, f: ExceptionFn) {
        self.set_exception_handler(6, f);
    }
    pub fn set_device_not_available(&self, f: ExceptionFn) {
        self.set_exception_handler(7, f);
    }
    pub fn set_double_fault(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(8, f);
    }
    pub fn set_invalid_tss(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(10, f);
    }
    pub fn set_segment_not_present(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(11, f);
    }
    pub fn set_stack_segment_fault(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(12, f);
    }
    pub fn set_general_protection_fault(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(13, f);
    }
    pub fn set_page_fault(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(14, f);
    }
    pub fn set_x87_floating_point_exception(&self, f: ExceptionFn) {
        self.set_exception_handler(16, f);
    }
    pub fn set_alignment_check(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(17, f);
    }
    pub fn set_machine_check(&self, f: ExceptionFn) {
        self.set_exception_handler(18, f);
    }
    pub fn set_simd_floating_point_exception(&self, f: ExceptionFn) {
        self.set_exception_handler(19, f);
    }
    pub fn set_virtualisation_exception(&self, f: ExceptionFn) {
        self.set_exception_handler(20, f);
    }
    pub fn set_security_exception(&self, f: ExceptionErrFn) {
        self.set_exception_err_handler(30, f);
    }
}

extern "C" {
    static interrupt_handlers: [*const u8; 256];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RegsFrame {
    pub cr2: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

#[no_mangle]
pub extern "C" fn isr_handler(
    int: usize,
    err: usize,
    frame: &mut InterruptFrame,
    regs: &mut RegsFrame,
) {
    let irqs = if int == 32 {
        // Special case for local timer int
        SHARED_IRQS.irqs[int].read_irq()
    } else {
        SHARED_IRQS.irqs[int].read_irq()
    };

    match &*irqs {
        IrqHandler::Exception(e) => {
            let f = *e;
            drop(irqs);

            f(frame, regs);
        }
        IrqHandler::ExceptionErr(e) => {
            let f = *e;
            drop(irqs);

            f(frame, regs, err as u64);
        }
        IrqHandler::Interrupt(e) => {
            let f = *e;
            drop(irqs);

            f();
        }
        IrqHandler::SharedInterrupt(e) => {
            for h in e.iter() {
                h();
            }

            drop(irqs);
        }
        IrqHandler::Missing => {
            drop(irqs);
        }
    }

    let ret_addr = VirtAddr(frame.ip as usize);

    if ret_addr.is_user() {
        crate::arch::signal::arch_int_check_signals(frame, regs);

        restore_user_fs();
    }

    end_of_int();
}

pub fn init() {
    let mut idt = IDT.lock_irq();

    unsafe {
        for (i, &h) in interrupt_handlers.iter().enumerate() {
            if h != core::ptr::null() {
                idt.set_handler(i, h as usize);
            }
        }
    }

    //Initialise exception handler routines
    SHARED_IRQS.set_divide_by_zero(divide_by_zero);
    SHARED_IRQS.set_debug(debug);
    SHARED_IRQS.set_non_maskable_interrupt(non_maskable_interrupt);
    SHARED_IRQS.set_breakpoint(breakpoint);
    SHARED_IRQS.set_overflow(overflow);
    SHARED_IRQS.set_bound_range_exceeded(bound_range_exceeded);
    SHARED_IRQS.set_invalid_opcode(invalid_opcode);
    SHARED_IRQS.set_device_not_available(device_not_available);
    SHARED_IRQS.set_double_fault(double_fault);
    SHARED_IRQS.set_invalid_tss(invalid_tss);
    SHARED_IRQS.set_segment_not_present(segment_not_present);
    SHARED_IRQS.set_stack_segment_fault(stack_segment_fault);
    SHARED_IRQS.set_general_protection_fault(general_protection_fault);
    SHARED_IRQS.set_page_fault(page_fault);
    SHARED_IRQS.set_x87_floating_point_exception(x87_floating_point_exception);
    SHARED_IRQS.set_alignment_check(alignment_check);
    SHARED_IRQS.set_machine_check(machine_check);
    SHARED_IRQS.set_simd_floating_point_exception(simd_floating_point_exception);
    SHARED_IRQS.set_virtualisation_exception(virtualisation_exception);
    SHARED_IRQS.set_security_exception(security_exception);

    idt.load();
}

pub fn init_ap() {
    let idt = IDT.lock_irq();
    idt.load();
}

pub fn has_handler(num: usize) -> bool {
    assert!(num <= 255);

    let irqs = SHARED_IRQS.irqs[num].read_irq();

    if let IrqHandler::Missing = *irqs {
        false
    } else {
        true
    }
}

pub fn remove_handler(num: usize) -> bool {
    assert!(num <= 255);
    let mut irqs = SHARED_IRQS.irqs[num].write_irq();

    match &mut *irqs {
        IrqHandler::Interrupt(_) | IrqHandler::ExceptionErr(_) | IrqHandler::Exception(_) => {
            *irqs = IrqHandler::Missing;

            true
        }
        _ => false,
    }
}

pub fn set_handler(num: usize, f: InterruptFn) {
    SHARED_IRQS.set_int_handler(num, f);
}

pub fn alloc_handler(f: InterruptFn) -> Option<usize> {
    SHARED_IRQS.alloc_int_handler(f)
}

pub fn set_user_handler(num: usize, f: InterruptFn) {
    assert!(num <= 255);

    unsafe {
        IDT.lock_irq().set_user(num, true);
    }

    SHARED_IRQS.set_int_handler(num, f);
}

static SHARED_IRQS: Irqs = {
    const MISSING: RwSpin<IrqHandler> = RwSpin::new(IrqHandler::Missing);
    Irqs {
        irqs: [MISSING; 256],
    }
};

pub fn add_shared_irq_handler(irq: usize, handler: SharedInterruptFn) {
    assert!(irq >= 32 && irq < 64, "invalid shared irq nr");

    SHARED_IRQS.set_shared_int_handler(irq, handler);
}

pub fn remove_shared_irq_handler(irq: usize, handler: SharedInterruptFn) {
    assert!(irq >= 32 && irq < 64, "invalid shared irq nr");

    SHARED_IRQS.remove_shared_int_handler(irq, handler);
}

fn divide_by_zero(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        println!("[ SIGFPE ] Task {} divide_by_zero error", task.tid());
        task.signal(syscall_defs::signal::SIGFPE);

        return;
    }
    loop {}
}

fn debug(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("INT: Debug exception! CPU: {}", crate::cpu_id());
    loop {}
}

fn non_maskable_interrupt(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("INT: Non Maskable Interrupt");
    loop {}
}

fn breakpoint(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("INT: Breakpoint!");
    loop {}
}

fn overflow(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!("[ SIGFPE ] Task {} overflow error", task.tid());
        task.signal(syscall_defs::signal::SIGFPE);

        return;
    }
    println!("Overflow error!");
    loop {}
}

fn bound_range_exceeded(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!("[ SIGSEGV ] Task {} bound range exceeded error", task.tid());
        task.signal(syscall_defs::signal::SIGSEGV);

        return;
    }
    println!("Bound Range Exceeded error!");
    loop {}
}

fn invalid_opcode(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!(
            "[ SIGILL ] Task {} invalid_opcode error {:#x}",
            task.tid(),
            frame.ip
        );
        task.signal(syscall_defs::signal::SIGILL);

        return;
    }
    println!(
        "Invalid Opcode error! task {} {:?} {}",
        crate::kernel::sched::current_id(),
        frame,
        unsafe { crate::CPU_ID }
    );
    loop {}
}

fn device_not_available(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("Device Not Available error!");
    loop {}
}

fn double_fault(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    println!("Double Fault error! 0x{:x}", err);
    loop {}
}

fn invalid_tss(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    println!("Invalid TSS error! 0x{:x}", err);
    loop {}
}

fn segment_not_present(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!("[ SIGSEGV ] Task {} segment_not_present error", task.tid());
        task.signal(syscall_defs::signal::SIGSEGV);

        return;
    }
    println!("Segment Not Present error 0x{:x}", err);
    loop {}
}

fn stack_segment_fault(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!("[ SIGSEGV ] Task {} stack_segment error", task.tid());
        task.signal(syscall_defs::signal::SIGSEGV);

        return;
    }
    println!("Stack Segment Failt error! 0x{:x}", err);
    loop {}
}

fn general_protection_fault(frame: &mut idt::InterruptFrame, regs: &mut RegsFrame, err: u64) {
    if frame.is_user() {
        let task = current_task_ref();

        logln!(
            "[ SIGBUS ] Task {} general_protecion error {:#x} {:?}",
            task.tid(),
            frame.ip,
            regs
        );
        task.signal(syscall_defs::signal::SIGBUS);

        return;
    }
    println!(
        "General Protection Fault error! 0x{:x} frame: {:?}",
        err,
        frame //unsafe { crate::CPU_ID }
    );
    loop {}
}

fn page_fault(frame: &mut idt::InterruptFrame, regs: &mut RegsFrame, err: u64) {
    let virt = VirtAddr(regs.cr2 as usize);

    let reason = PageFaultReason::from_bits_truncate(err as usize);

    if virt.is_user() {
        // page fault originated in userspace
        // let the task try handle it
        let task = current_task();

        //println!("user pagefault {:#x} {} {:?} pid: {}", frame.ip, virt, reason, task.tid());
        if task.handle_pagefault(reason, virt) {
            return;
        } else {
            logln!(
                "[ SIGSEGV ] Task {} page_fault error addr: {}, ip: {:#x}, err: {}",
                task.tid(),
                virt,
                frame.ip,
                err
            );
        }
    //println!("user pagefault failed");
    } else if reason.contains(PageFaultReason::PRESENT | PageFaultReason::WRITE) {
        // page fault caused by write access and page was present
        // try to notify cache to mark page dirty and enable writeable flag

        if let Some(p) = virt.to_phys_pagewalk() {
            if let Some(i) = p.to_phys_page() {
                if let Some(h) = i.page_item() {
                    h.notify_dirty(&h, None);

                    return;
                }
            }
        }
    }

    if VirtAddr(frame.ip as usize).is_user() {
        let task = current_task();
        task.signal(syscall_defs::signal::SIGSEGV);

        return;
    }

    println!(
        "PAGE FAULT! 0x{:x} CPU: {}, rip: {:?} virt: {}",
        err,
        unsafe { crate::CPU_ID },
        frame,
        virt
    );
    loop {}
}

fn x87_floating_point_exception(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        println!("[ SIGSEGV ] Task {} x87_floating_point error", task.tid());
        task.signal(syscall_defs::signal::SIGFPE);

        return;
    }
    println!("x87 Floating Point Exception!");
    loop {}
}

fn alignment_check(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    if frame.is_user() {
        let task = current_task_ref();

        println!("[ SIGBUS ] Task {} alignment_check error", task.tid());
        task.signal(syscall_defs::signal::SIGBUS);

        return;
    }
    println!("Alignment Check error! 0x{:x}", err);
    loop {}
}

fn machine_check(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("Machine Check error");
    loop {}
}

fn simd_floating_point_exception(frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    if frame.is_user() {
        let task = current_task_ref();

        println!("[ SIGFPE ] Task {} simd_floating_point error", task.tid());
        task.signal(syscall_defs::signal::SIGFPE);

        return;
    }
    println!("SIMD Floating Point Exception!");
    loop {}
}

fn virtualisation_exception(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame) {
    println!("Virtualisation Exception!");
    loop {}
}

fn security_exception(_frame: &mut idt::InterruptFrame, _regs: &mut RegsFrame, err: u64) {
    println!("Security Exception! 0x{:x}", err);
    loop {}
}
