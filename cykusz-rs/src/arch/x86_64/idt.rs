use alloc::vec::Vec;

use paste::paste;
use spin::RwLock;

use crate::arch::raw::idt;
use crate::arch::x86_64::int::end_of_int;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::task::vm::PageFaultReason;

static IDT: Spin<idt::Idt> = Spin::new(idt::Idt::new());

struct SharedIrq {
    irqs: [Vec<fn() -> bool>; 32],
}

pub fn init() {
    let mut idt = IDT.lock();
    //Initialise exception handler routines
    idt.set_divide_by_zero(divide_by_zero);
    idt.set_debug(debug);
    idt.set_non_maskable_interrupt(non_maskable_interrupt);
    idt.set_breakpoint(breakpoint);
    idt.set_overflow(overflow);
    idt.set_bound_range_exceeded(bound_range_exceeded);
    idt.set_invalid_opcode(invalid_opcode);
    idt.set_device_not_available(device_not_available);
    idt.set_double_fault(double_fault);
    idt.set_invalid_tss(invalid_tss);
    idt.set_segment_not_present(segment_not_present);
    idt.set_stack_segment_fault(stack_segment_fault);
    idt.set_general_protection_fault(general_protection_fault);
    idt.set_page_fault(page_fault);
    idt.set_x87_floating_point_exception(x87_floating_point_exception);
    idt.set_alignment_check(alignment_check);
    idt.set_machine_check(machine_check);
    idt.set_simd_floating_point_exception(simd_floating_point_exception);
    idt.set_virtualisation_exception(virtualisation_exception);
    idt.set_security_exception(security_exception);

    unsafe {
        idt.set_handler(32, shared_32);
        idt.set_handler(33, shared_33);
        idt.set_handler(34, shared_34);
        idt.set_handler(35, shared_35);
        idt.set_handler(36, shared_36);
        idt.set_handler(37, shared_37);
        idt.set_handler(38, shared_38);
        idt.set_handler(39, shared_39);
        idt.set_handler(40, shared_40);
        idt.set_handler(41, shared_41);
        idt.set_handler(42, shared_42);
        idt.set_handler(43, shared_43);
        idt.set_handler(44, shared_44);
        idt.set_handler(45, shared_45);
        idt.set_handler(46, shared_46);
        idt.set_handler(47, shared_47);
        idt.set_handler(48, shared_48);
        idt.set_handler(49, shared_49);
        idt.set_handler(50, shared_50);
        idt.set_handler(51, shared_51);
        idt.set_handler(52, shared_52);
        idt.set_handler(53, shared_53);
        idt.set_handler(54, shared_54);
        idt.set_handler(55, shared_55);
        idt.set_handler(56, shared_56);
        idt.set_handler(57, shared_57);
        idt.set_handler(58, shared_58);
        idt.set_handler(59, shared_59);
        idt.set_handler(60, shared_60);
        idt.set_handler(61, shared_61);
        idt.set_handler(62, shared_62);
        idt.set_handler(63, shared_63);
    }

    idt.load();
}

pub fn set_handler(num: usize, f: idt::ExceptionHandlerFn) {
    assert!(num < 32 || num >= 64);
    unsafe {
        let mut idt = IDT.lock();
        idt.set_handler(num, f);
    }
}

pub fn has_handler(num: usize) -> bool {
    assert!(num <= 255);
    if num < 32 || num >= 64 {
        let idt = IDT.lock();

        idt.has_handler(num)
    } else {
        let sh = SHARED_IRQS.read();

        return !sh.irqs[num - 32].is_empty();
    }
}

pub fn remove_handler(num: usize) {
    assert!(num <= 255);
    let mut idt = IDT.lock();

    idt.remove_handler(num);
}

pub fn set_user_handler(num: usize, f: idt::ExceptionHandlerFn) {
    assert!(num <= 255);
    unsafe {
        let mut idt = IDT.lock();
        idt.set_user_handler(num, f);
    }
}

static SHARED_IRQS: RwLock<SharedIrq> = RwLock::new(SharedIrq {
    irqs: [
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ],
});

fn handle_shared_irq(irq: u32) {
    let idx = irq - 32;

    let sh = SHARED_IRQS.read();

    for h in sh.irqs[idx as usize].iter() {
        h();
    }

    end_of_int();
}

pub fn add_shared_irq_handler(irq: usize, handler: fn() -> bool) {
    let mut sh = SHARED_IRQS.write();

    assert!(irq >= 32 && irq < 64, "invalid shared irq nr");

    let idx = irq - 32;

    sh.irqs[idx].push(handler);
}

pub fn remove_shared_irq_handler(irq: usize, handler: fn() -> bool) {
    let mut sh = SHARED_IRQS.write();

    assert!(irq >= 32 && irq < 64, "invalid shared irq nr");

    let idx = irq - 32;

    if let Some(i) = sh.irqs[idx].iter().enumerate().find_map(|(i, e)| {
        if *e == handler {
            return Some(i);
        } else {
            None
        }
    }) {
        sh.irqs[idx].remove(i);
    }
}

macro_rules! def_shared {
    ($num:expr) => {
        paste! {
            extern "x86-interrupt" fn [<shared_ $num>](_frame: &mut idt::ExceptionStackFrame) {
                handle_shared_irq($num);
            }
        }
    };
}

def_shared!(32);
def_shared!(33);
def_shared!(34);
def_shared!(35);
def_shared!(36);
def_shared!(37);
def_shared!(38);
def_shared!(39);
def_shared!(40);
def_shared!(41);
def_shared!(42);
def_shared!(43);
def_shared!(44);
def_shared!(45);
def_shared!(46);
def_shared!(47);
def_shared!(48);
def_shared!(49);
def_shared!(50);
def_shared!(51);
def_shared!(52);
def_shared!(53);
def_shared!(54);
def_shared!(55);
def_shared!(56);
def_shared!(57);
def_shared!(58);
def_shared!(59);
def_shared!(60);
def_shared!(61);
def_shared!(62);
def_shared!(63);

#[allow(unused)]
extern "x86-interrupt" fn dummy(_frame: &mut idt::ExceptionStackFrame) {
    println!("Dummy int");
    crate::arch::int::end_of_int();
}

extern "x86-interrupt" fn divide_by_zero(_frame: &mut idt::ExceptionStackFrame) {
    println!("Divide By Zero error!");
    loop {}
}

extern "x86-interrupt" fn debug(_frame: &mut idt::ExceptionStackFrame) {
    unsafe {
        println!("INT: Debug exception! CPU: {}", crate::CPU_ID);
    }
    loop {}
}

extern "x86-interrupt" fn non_maskable_interrupt(_frame: &mut idt::ExceptionStackFrame) {
    println!("INT: Non Maskable Interrupt");
    loop {}
}

extern "x86-interrupt" fn breakpoint(_frame: &mut idt::ExceptionStackFrame) {
    println!("INT: Breakpoint!");
    loop {}
}

extern "x86-interrupt" fn overflow(_frame: &mut idt::ExceptionStackFrame) {
    println!("Overflow error!");
    loop {}
}

extern "x86-interrupt" fn bound_range_exceeded(_frame: &mut idt::ExceptionStackFrame) {
    println!("Bound Range Exceeded error!");
    loop {}
}

extern "x86-interrupt" fn invalid_opcode(_frame: &mut idt::ExceptionStackFrame) {
    println!(
        "Invalid Opcode error! task {} {:?} {}",
        crate::kernel::sched::current_id(),
        _frame,
        unsafe { crate::CPU_ID }
    );
    loop {}
}

extern "x86-interrupt" fn device_not_available(_frame: &mut idt::ExceptionStackFrame) {
    println!("Device Not Available error!");
    loop {}
}

extern "x86-interrupt" fn double_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Double Fault error! 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn invalid_tss(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Invalid TSS error! 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn segment_not_present(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Segment Not Present error 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn stack_segment_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Stack Segment Failt error! 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn general_protection_fault(
    _frame: &mut idt::ExceptionStackFrame,
    err: u64,
) {
    unsafe {
        println!(
            "General Protection Fault error! 0x{:x} CPU: {}",
            err,
            crate::CPU_ID
        );
    }
    loop {}
}

extern "x86-interrupt" fn page_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
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
                    h.notify_dirty(&h);

                    return;
                }
            }
        }
    }

    crate::bochs();
    println!(
        "PAGE FAULT! 0x{:x} CPU: {}, rip: {:?}",
        err,
        unsafe { crate::CPU_ID },
        _frame
    );
    loop {}
}

extern "x86-interrupt" fn x87_floating_point_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("x87 Floating Point Exception!");
    loop {}
}

extern "x86-interrupt" fn alignment_check(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Alignment Check error! 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn machine_check(_frame: &mut idt::ExceptionStackFrame) {
    println!("Machine Check error");
    loop {}
}

extern "x86-interrupt" fn simd_floating_point_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("SIMD Floating Point Exception!");
    loop {}
}

extern "x86-interrupt" fn virtualisation_exception(_frame: &mut idt::ExceptionStackFrame) {
    println!("Virtualisation Exception!");
    loop {}
}

extern "x86-interrupt" fn security_exception(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("Security Exception! 0x{:x}", err);
    loop {}
}
