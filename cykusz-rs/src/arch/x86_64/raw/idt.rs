use core::fmt;

use crate::arch::raw::descriptor as dsc;
use crate::arch::raw::segmentation::SegmentSelector;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct IdtEntry {
    pub offset_1: u16,
    pub selector: SegmentSelector,
    pub ist: u8,
    pub type_attr: dsc::Flags,
    pub offset_2: u16,
    pub offset_3: u32,
    pub zero: u32,
}

#[repr(C)]
pub struct InterruptFrame {
    pub ip: u64,
    pub cs: u64,
    pub cf: u64,
    pub sp: u64,
    pub ss: u64,
}

impl InterruptFrame {
    pub fn is_user(&self) -> bool {
        SegmentSelector::from_raw(self.cs as u16).contains(SegmentSelector::RPL_3)
    }
}

impl ::core::fmt::Debug for InterruptFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r#"ExceptionStackFrame
    ip: 0x{:x}
    cs: 0x{:x}
    cf: 0x{:x}
    sp: 0x{:x}
    ss: 0x{:x}"#,
            self.ip, self.cs, self.cf, self.sp, self.ss
        )
    }
}

#[allow(unused)]
macro_rules! int {
    ( $x:expr) => {
        {
            llvm_asm!("int $0" :: "N"($x));
        }
    };
}

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

    pub fn set_user(&mut self, user: bool) {
        if user {
            self.type_attr.remove(dsc::Flags::SYS_RING0_INTERRUPT_GATE);
            self.type_attr.insert(dsc::Flags::SYS_RING3_INTERRUPT_GATE);
        } else {
            self.type_attr.remove(dsc::Flags::SYS_RING3_INTERRUPT_GATE);
            self.type_attr.insert(dsc::Flags::SYS_RING0_INTERRUPT_GATE);
        }
    }
}

#[repr(C)]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Idt {
        Idt {
            entries: [IdtEntry::MISSING; 256],
        }
    }

    pub fn has_handler(&self, num: usize) -> bool {
        self.entries[num] != IdtEntry::MISSING
    }

    pub fn remove_handler(&mut self, idx: usize) {
        self.entries[idx] = IdtEntry::MISSING;
    }

    pub unsafe fn set_handler(&mut self, idx: usize, f: usize) {
        self.entries[idx].set_handler(
            f as u64,
            crate::arch::gdt::ring0_cs(),
            dsc::Flags::SYS_RING0_INTERRUPT_GATE,
        );
    }

    pub unsafe fn set_user(&mut self, idx: usize, user: bool) {
        self.entries[idx].set_user(user);
    }

    pub fn load(&self) {
        let mut idtr = dsc::DescriptorTablePointer::<IdtEntry>::empty();
        idtr.init(&self.entries[..]);
        unsafe {
            dsc::lidt(&idtr);
        }
    }
}
