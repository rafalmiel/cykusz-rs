use arch::raw::segmentation::SegmentSelector;
use arch::raw::descriptor as dsc;

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

pub type HandlerFun = extern "x86-interrupt" fn (&mut ExceptionStackFrame);

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

    pub fn set_handler(&mut self, f: HandlerFun, selector: SegmentSelector, flags: dsc::Flags) {
        let ptr = f as u64;
        self.offset_1 = ptr as u16;
        self.selector = selector;
        self.ist = 0;
        self.type_attr = flags;
        self.offset_2 = (ptr >> 16) as u16;
        self.offset_3 = (ptr >> 32) as u32;
        self.zero = 0;
    }
}
