use arch::raw::segmentation::SegmentSelector;
use arch::raw::descriptor as dsc;
use arch::raw::segmentation::cs;

use core::fmt;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct IdtEntry {
    pub offset_1: u16,
    pub selector: SegmentSelector,
    pub ist: u8,
    pub type_attr: dsc::Flags,
    pub offset_2: u16,
    pub offset_3: u32,
    pub zero: u32
}

#[repr(C)]
pub struct ExceptionStackFrame {
    pub ip: u64,
    pub cs: u64,
    pub cf: u64,
    pub sp: u64,
    pub ss: u64,
}

impl ::core::fmt::Debug for ExceptionStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"ExceptionStackFrame
    ip: 0x{:x}
    cs: 0x{:x}
    cf: 0x{:x}
    sp: 0x{:x}
    ss: 0x{:x}"#, self.ip, self.cs, self.cf, self.sp, self.ss)
    }
}

macro_rules! int {
    ( $x:expr) => {
        {
            asm!("int $0" :: "N"($x));
        }
    };
}

pub type HandlerFn =       extern "x86-interrupt" fn (&mut ExceptionStackFrame);
pub type HandlerFnErrCode = extern "x86-interrupt" fn (&mut ExceptionStackFrame, err_code: u64);

impl IdtEntry {
    pub const MISSING: IdtEntry = IdtEntry {
        offset_1: 0,
        selector: SegmentSelector::from_raw(0),
        ist: 0,
        type_attr: dsc::Flags::MISSING,
        offset_2: 0,
        offset_3: 0,
        zero: 0,
    };

    pub const fn new(ptr: usize, selector: SegmentSelector, flags: dsc::Flags) -> IdtEntry {
        IdtEntry {
            offset_1: ptr as u16,
            selector,
            ist: 0,
            type_attr: flags,
            offset_2: (ptr >> 16) as u16,
            offset_3: (ptr >> 32) as u32,
            zero: 0,
        }
    }

    pub fn set_handler(&mut self, ptr: u64, selector: SegmentSelector, flags: dsc::Flags) {
        self.offset_1 = ptr as u16;
        self.selector = selector;
        self.ist = 0;
        self.type_attr = flags;
        self.offset_2 = (ptr >> 16) as u16;
        self.offset_3 = (ptr >> 32) as u32;
        self.zero = 0;
    }

    pub fn set_handler_fn(&mut self, f: HandlerFn, selector: SegmentSelector, flags: dsc::Flags) {
        self.set_handler(f as u64, selector, flags);
    }

    pub fn set_handler_fn_err(&mut self, f: HandlerFnErrCode, selector: SegmentSelector, flags: dsc::Flags) {
        self.set_handler(f as u64, selector, flags);
    }
}

#[repr(C)]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Idt {
        Idt {
            entries: [IdtEntry::MISSING; 256]
        }
    }

    unsafe fn set_handler(&mut self, idx: usize, f: HandlerFn) {
        unsafe {
           self.entries[idx].set_handler_fn(f, cs(), dsc::Flags::SYS_RING0_INTERRUPT_GATE);
        }
    }

    unsafe fn set_handler_err(&mut self, idx: usize, f: HandlerFnErrCode) {
        unsafe {
            self.entries[idx].set_handler_fn_err(f, cs(), dsc::Flags::SYS_RING0_INTERRUPT_GATE);
        }
    }

    pub fn load(&'static self) {
        let mut idtr = dsc::DescriptorTablePointer::<IdtEntry>::empty();
        idtr.init(&self.entries[..]);
        unsafe {
            dsc::lidt(&idtr);
        }
    }

    pub fn set_divide_by_zero(&mut self, f: HandlerFn) {
        unsafe { self.set_handler(0, f) };
    }
    pub fn set_debug(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(1, f) };
    }
    pub fn set_non_maskable_interrupt(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(2, f) };
    }
    pub fn set_breakpoint(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(3, f) };
    }
    pub fn set_overflow(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(4, f) };
    }
    pub fn set_bound_range_exceeded(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(5, f) };
    }
    pub fn set_invalid_opcode(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(6, f) };
    }
    pub fn set_device_not_available(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(7, f) };
    }
    pub fn set_double_fault(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(8, f) };
    }
    pub fn set_invalid_tss(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(10, f) };
    }
    pub fn set_segment_not_present(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(11, f) };
    }
    pub fn set_stack_segment_fault(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(12, f) };
    }
    pub fn set_general_protection_fault(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(13, f) };
    }
    pub fn set_page_fault(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(14, f) };
    }
    pub fn set_x87_floating_point_exception(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(16, f) };
    }
    pub fn set_alignment_check(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(17, f) };
    }
    pub fn set_machine_check(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(18, f) };
    }
    pub fn set_simd_floating_point_exception(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(19, f) };
    }
    pub fn set_virtualisation_exception(&mut self, f: HandlerFn) {
        unsafe {self.set_handler(20, f) };
    }
    pub fn set_security_exception(&mut self, f: HandlerFnErrCode) {
        unsafe {self.set_handler_err(30, f) };
    }
}
