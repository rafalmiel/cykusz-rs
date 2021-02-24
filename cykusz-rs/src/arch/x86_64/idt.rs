use alloc::vec::Vec;

use crate::arch::raw::idt;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::tls::restore_user_fs;
use crate::arch::x86_64::int::end_of_int;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::current_task;
use crate::kernel::sync::RwSpin;
use crate::kernel::sync::Spin;
use crate::kernel::task::vm::PageFaultReason;

static IDT: Spin<idt::Idt> = Spin::new(idt::Idt::new());

pub type ExceptionFn = fn(&mut ExceptionStackFrame);
pub type ExceptionErrFn = fn(&mut ExceptionStackFrame, u64);
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
    irqs: RwSpin<[IrqHandler; 256]>,
}

impl Irqs {
    fn set_exception_handler(&self, idx: usize, f: ExceptionFn) {
        let mut irqs = self.irqs.write_irq();

        match irqs[idx] {
            IrqHandler::Missing => {
                irqs[idx] = IrqHandler::Exception(f);
            }
            _ => {
                panic!("Exception handler already exists");
            }
        }
    }

    fn set_exception_err_handler(&self, idx: usize, f: ExceptionErrFn) {
        let mut irqs = self.irqs.write_irq();

        match irqs[idx] {
            IrqHandler::Missing => {
                irqs[idx] = IrqHandler::ExceptionErr(f);
            }
            _ => {
                panic!("ExceptionErr handler already exists");
            }
        }
    }

    fn set_int_handler(&self, idx: usize, f: InterruptFn) {
        let mut irqs = self.irqs.write_irq();

        match irqs[idx] {
            IrqHandler::Missing => {
                irqs[idx] = IrqHandler::Interrupt(f);
            }
            _ => {
                panic!("Interrupt handler already exists");
            }
        }
    }

    fn set_shared_int_handler(&self, idx: usize, f: SharedInterruptFn) {
        let mut irqs = self.irqs.write_irq();

        match &mut irqs[idx] {
            IrqHandler::Missing => {
                let mut v = Vec::<SharedInterruptFn>::new();
                v.push(f);
                irqs[idx] = IrqHandler::SharedInterrupt(v);
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
        let mut irqs = self.irqs.write_irq();

        if let IrqHandler::SharedInterrupt(h) = &mut irqs[idx] {
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

#[no_mangle]
pub extern "C" fn isr_handler(int: usize, err: usize, frame: &mut ExceptionStackFrame) {
    let irqs = SHARED_IRQS.irqs.read();

    match &irqs[int] {
        IrqHandler::Exception(e) => {
            e(frame);
        }
        IrqHandler::ExceptionErr(e) => {
            e(frame, err as u64);
        }
        IrqHandler::Interrupt(e) => {
            e();
        }
        IrqHandler::SharedInterrupt(e) => {
            for h in e.iter() {
                h();
            }
        }
        IrqHandler::Missing => {}
    }

    drop(irqs);

    let ret_addr = VirtAddr(frame.ip as usize);

    if ret_addr.is_user() {
        restore_user_fs();
    }

    end_of_int();
}

pub fn init() {
    let mut idt = IDT.lock();

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
    let idt = IDT.lock();
    idt.load();
}

pub fn has_handler(num: usize) -> bool {
    assert!(num <= 255);

    let irqs = SHARED_IRQS.irqs.read();

    if let IrqHandler::Missing = irqs[num] {
        false
    } else {
        true
    }
}

pub fn remove_handler(num: usize) -> bool {
    assert!(num <= 255);
    let mut irqs = SHARED_IRQS.irqs.write_irq();

    match &irqs[num] {
        IrqHandler::Interrupt(_) | IrqHandler::ExceptionErr(_) | IrqHandler::Exception(_) => {
            irqs[num] = IrqHandler::Missing;

            true
        }
        _ => false,
    }
}

pub fn set_user_handler(num: usize, f: InterruptFn) {
    assert!(num <= 255);

    unsafe {
        IDT.lock_irq().set_user(num, true);
    }

    SHARED_IRQS.set_int_handler(num, f);
}

static SHARED_IRQS: Irqs = {
    const MISSING: IrqHandler = IrqHandler::Missing;
    Irqs {
        irqs: RwSpin::new([MISSING; 256]),
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

fn divide_by_zero(_frame: &mut idt::ExceptionStackFrame) {
    println!("Divide By Zero error!");
    loop {}
}

fn debug(_frame: &mut idt::ExceptionStackFrame) {
    unsafe {
        println!("INT: Debug exception! CPU: {}", crate::CPU_ID);
    }
    loop {}
}

fn non_maskable_interrupt(_frame: &mut idt::ExceptionStackFrame) {
    println!("INT: Non Maskable Interrupt");
    loop {}
}

fn breakpoint(_frame: &mut idt::ExceptionStackFrame) {
    println!("INT: Breakpoint!");
    loop {}
}

fn overflow(_frame: &mut idt::ExceptionStackFrame) {
    println!("Overflow error!");
    loop {}
}

fn bound_range_exceeded(_frame: &mut idt::ExceptionStackFrame) {
    println!("Bound Range Exceeded error!");
    loop {}
}

fn invalid_opcode(_frame: &mut idt::ExceptionStackFrame) {
    println!(
        "Invalid Opcode error! task {} {:?} {}",
        crate::kernel::sched::current_id(),
        _frame,
        unsafe { crate::CPU_ID }
    );
    loop {}
}

fn device_not_available(_frame: &mut idt::ExceptionStackFrame) {
    println!("Device Not Available error!");
    loop {}
}

fn double_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Double Fault error! 0x{:x}", err);
    loop {}
}

fn invalid_tss(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Invalid TSS error! 0x{:x}", err);
    loop {}
}

fn segment_not_present(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Segment Not Present error 0x{:x}", err);
    loop {}
}

fn stack_segment_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Stack Segment Failt error! 0x{:x}", err);
    loop {}
}

fn general_protection_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    unsafe {
        println!(
            "General Protection Fault error! 0x{:x} CPU: {}",
            err,
            crate::CPU_ID
        );
    }
    loop {}
}

fn page_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    let virt = VirtAddr(unsafe { crate::arch::raw::ctrlregs::cr2() });

    let reason = PageFaultReason::from_bits_truncate(err as usize);

    if virt.is_user() {
        // page fault originated in userspace
        // let the task try handle it

        let task = current_task();

        //println!("user pagefault {} {:?}", virt, reason);
        if task.handle_pagefault(reason, virt) {
            return;
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

    crate::bochs();
    println!(
        "PAGE FAULT! 0x{:x} CPU: {}, rip: {:?} virt: {}",
        err,
        unsafe { crate::CPU_ID },
        _frame,
        virt
    );
    loop {}
}

fn x87_floating_point_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("x87 Floating Point Exception!");
    loop {}
}

fn alignment_check(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Alignment Check error! 0x{:x}", err);
    loop {}
}

fn machine_check(_frame: &mut idt::ExceptionStackFrame) {
    println!("Machine Check error");
    loop {}
}

fn simd_floating_point_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("SIMD Floating Point Exception!");
    loop {}
}

fn virtualisation_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("Virtualisation Exception!");
    loop {}
}

fn security_exception(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Security Exception! 0x{:x}", err);
    loop {}
}
