use arch::raw::idt;

use spin::Mutex;

static IDT: Mutex<idt::Idt> = Mutex::new(idt::Idt::new());

pub fn init() {
    let mut idt = IDT.lock();
    //Initialise exception handler routines
    idt.set_divide_by_zero(               divide_by_zero);
    idt.set_debug(                        debug);
    idt.set_non_maskable_interrupt(       non_maskable_interrupt);
    idt.set_breakpoint(                   breakpoint);
    idt.set_overflow(                     overflow);
    idt.set_bound_range_exceeded(         bound_range_exceeded);
    idt.set_invalid_opcode(               invalid_opcode);
    idt.set_device_not_available(         device_not_available);
    idt.set_double_fault(                 double_fault);
    idt.set_invalid_tss(                  invalid_tss);
    idt.set_segment_not_present(          segment_not_present);
    idt.set_stack_segment_fault(          stack_segment_fault);
    idt.set_general_protection_fault(     general_protection_fault);
    idt.set_page_fault(                   page_fault);
    idt.set_x87_floating_point_exception( x87_floating_point_exception);
    idt.set_alignment_check(              alignment_check);
    idt.set_machine_check(                machine_check);
    idt.set_simd_floating_point_exception(simd_floating_point_exception);
    idt.set_virtualisation_exception(     virtualisation_exception);
    idt.set_security_exception(           security_exception);
    for i in 32..256 {
        unsafe {
            idt.set_handler(i, dummy);
        }
    }
    idt.load();

}

pub fn set_handler(num: usize, f: idt::HandlerFn) {
    assert!(num <= 255);
    unsafe {
        IDT.lock().set_handler(num, f);
    }
}


extern "x86-interrupt" fn dummy(_frame: &mut idt::ExceptionStackFrame) {
    println!("Dummy int");
    ::arch::int::end_of_int();
}

extern "x86-interrupt" fn divide_by_zero(_frame: &mut idt::ExceptionStackFrame) {
    println!("Divide By Zero error!");
    loop {}
}

extern "x86-interrupt" fn debug(_frame: &mut idt::ExceptionStackFrame) {
    println!("INT: Debug exception!");
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
    println!("Invalid Opcode error! {:?}", _frame);
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

extern "x86-interrupt" fn general_protection_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("General Protection Fault error! 0x{:x}", err);
    loop {}
}

extern "x86-interrupt" fn page_fault(_frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("PAGE FAULT! 0x{:x}", err);
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

extern "x86-interrupt" fn int80_handler(frame: &mut idt::ExceptionStackFrame) {
    println!("INT 80!!!");
    println!("{:?}", frame);
}
