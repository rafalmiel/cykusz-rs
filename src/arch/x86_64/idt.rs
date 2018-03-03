use arch::raw::idt;
use arch::raw::descriptor as dsc;
use arch::gdt;

static mut IDT : idt::Idt = idt::Idt::new();

pub fn init() {
    unsafe {
        IDT.set_divide_by_zero(               divide_by_zero);
        IDT.set_debug(                        debug);
        IDT.set_non_maskable_interrupt(       non_maskable_interrupt);
        IDT.set_breakpoint(                   breakpoint);
        IDT.set_overflow(                     overflow);
        IDT.set_bound_range_exceeded(         bound_range_exceeded);
        IDT.set_invalid_opcode(               invalid_opcode);
        IDT.set_device_not_available(         device_not_available);
        IDT.set_double_fault(                 double_fault);
        IDT.set_invalid_tss(                  invalid_tss);
        IDT.set_segment_not_present(          segment_not_present);
        IDT.set_stack_segment_fault(          stack_segment_fault);
        IDT.set_general_protection_fault(     general_protection_fault);
        IDT.set_page_fault(                   page_fault);
        IDT.set_x87_floating_point_exception( x87_floating_point_exception);
        IDT.set_alignment_check(              alignment_check);
        IDT.set_machine_check(                machine_check);
        IDT.set_simd_floating_point_exception(simd_floating_point_exception);
        IDT.set_virtualisation_exception(     virtualisation_exception);
        IDT.set_security_exception(           security_exception);

        IDT.load();
    }

    println!("IDT Initialised");
    unsafe {
        //test pagefault..
        *(0x100 as *mut u64) = 33;
    }
}

extern "x86-interrupt" fn divide_by_zero(frame: &mut idt::ExceptionStackFrame) {
    println!("DIVIDE BY ZERO EXCEPTION");

}
extern "x86-interrupt" fn debug(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn non_maskable_interrupt(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn breakpoint(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn overflow(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn bound_range_exceeded(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn invalid_opcode(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn device_not_available(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn double_fault(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn invalid_tss(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn segment_not_present(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn stack_segment_fault(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn general_protection_fault(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn page_fault(frame: &mut idt::ExceptionStackFrame, err: u64) {
    println!("PAGE FAULT! 0x{:x}", err);
    loop{}

}
extern "x86-interrupt" fn x87_floating_point_exception(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn alignment_check(frame: &mut idt::ExceptionStackFrame, err: u64) {

}
extern "x86-interrupt" fn machine_check(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn simd_floating_point_exception(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn virtualisation_exception(frame: &mut idt::ExceptionStackFrame) {

}
extern "x86-interrupt" fn security_exception(frame: &mut idt::ExceptionStackFrame, err: u64) {

}

extern "x86-interrupt" fn int80_handler(frame: &mut idt::ExceptionStackFrame) {
    println!("INT 80!!!");
    println!("{:?}", frame);
}
