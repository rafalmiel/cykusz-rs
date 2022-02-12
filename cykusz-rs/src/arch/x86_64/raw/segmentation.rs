use core::arch::asm;

// Specifies which element to load into a segment from
// descriptor tables (i.e., is a index to LDT or GDT table
// with some additional flags).
//
// See Intel 3a, Section 3.4.2 "Segment Selectors"
bitflags! {
    pub struct SegmentSelector: u16 {
        /// Requestor Privilege Level
        const RPL_0 = 0b00;
        const RPL_1 = 0b01;
        const RPL_2 = 0b10;
        const RPL_3 = 0b11;

        /// Table Indicator (TI) 0 means GDT is used.
        const TI_GDT = 0 << 2;
        /// Table Indicator (TI) 1 means LDT is used.
        const TI_LDT = 1 << 2;
    }
}

impl SegmentSelector {
    /// Create a new SegmentSelector
    ///
    /// # Arguments
    ///  * `index` index in GDT or LDT array.
    ///
    pub const fn new(index: u16, rpl: SegmentSelector) -> SegmentSelector {
        SegmentSelector {
            bits: index << 3 | rpl.cbits(),
        }
    }

    pub const fn cbits(&self) -> u16 {
        self.bits
    }

    pub const fn from_raw(bits: u16) -> SegmentSelector {
        SegmentSelector { bits }
    }
}

pub fn cs() -> SegmentSelector {
    let segment: u16;
    unsafe { asm!("mov {segment:x}, cs", segment = lateout(reg) segment) }
    SegmentSelector::from_raw(segment)
}

pub fn ds() -> SegmentSelector {
    let segment: u16;
    unsafe { asm!("mov {segment:x}, ds", segment = lateout(reg) segment) }
    SegmentSelector::from_raw(segment)
}

pub unsafe fn set_cs(sel: SegmentSelector) {
    asm!("pushq {};\
        leaq 1f(%rip), %rax;\
        pushq %rax;\
        lretq;\
        1:", in(reg) sel.bits() as usize, options(att_syntax))
}

/// Reload stack segment register.
pub unsafe fn load_ss(sel: SegmentSelector) {
    asm!("mov ss, {0:x} ", in(reg) sel.bits());
}

/// Reload data segment register.
pub unsafe fn load_ds(sel: SegmentSelector) {
    asm!("mov ds, {0:x} ", in(reg) sel.bits());
}

/// Reload es segment register.
pub unsafe fn load_es(sel: SegmentSelector) {
    asm!("mov es, {0:x} ", in(reg) sel.bits());
}

/// Reload fs segment register.
pub unsafe fn load_fs(sel: SegmentSelector) {
    asm!("mov fs, {0:x} ", in(reg) sel.bits());
}

/// Reload gs segment register.
pub unsafe fn load_gs(sel: SegmentSelector) {
    asm!("mov gs, {0:x} ", in(reg) sel.bits());
}
