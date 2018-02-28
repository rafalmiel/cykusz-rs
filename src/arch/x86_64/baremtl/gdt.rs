use core::mem;

bitflags! {
    pub struct GdtAccessFlags: u8 {
        const BLANK = 0;
        // Presetnt BIT
        const PRESENT = 1 << 7;

        //Privilige bits
        const PRIV_RING0 = 0b00 << 5;
        const PRIV_RING1 = 0b01 << 5;
        const PRIV_RING2 = 0b10 << 5;
        const PRIV_RING3 = 0b11 << 5;

        const SEGMENT = 1 << 4;

        // Executable bit
        const EXECUTABLE = 1 << 3;

        // Confirming bit for code, direction bit for data
        const CONFORMING = 1 << 2;
        const GROW_DOWN = 1 << 2;

        // RD for code, WR for data
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 1;

        // Accessed bit
        const ACCESSED = 1;

        const RING0_CODE = Self::PRESENT.bits | Self::PRIV_RING0.bits | Self::SEGMENT.bits | Self::EXECUTABLE.bits | Self::READABLE.bits;
        const RING0_DATA = Self::PRESENT.bits | Self::PRIV_RING0.bits | Self::SEGMENT.bits | Self::WRITABLE.bits;
    }
}

bitflags! {
    pub struct GdtFlags: u8 {
        const BLANK = 0;

        // Granularity bit
        const PAGE_GRANULARITY = 1 << 7;
        const LONG_MODE = 1 << 5;
    }
}

impl GdtAccessFlags {
    pub const fn cbits(&self) -> u8 {
        return self.bits;
    }
}

impl GdtFlags {
    pub const fn cbits(&self) -> u8 {
        return self.bits;
    }
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct GdtEntry {
    pub limitl: u16,
    pub offsetl: u16,
    pub offsetm: u8,
    pub access: u8,
    pub flags_limith: u8,
    pub offseth: u8
}

#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u64,
}

impl DescriptorTablePointer {
    pub fn init(&mut self, e_slice: &[GdtEntry]) {
        self.limit = (e_slice.len() * mem::size_of::<GdtEntry>() - 1) as u16;
        self.base = e_slice.as_ptr() as u64;
    }

    pub const fn empty() -> Self {
        DescriptorTablePointer {
            limit: 0,
            base: 0,
        }
    }
}

impl GdtEntry {
    pub const fn null() -> Self {
        Self::new(GdtAccessFlags::BLANK, GdtFlags::BLANK)
    }

    pub const fn new(access: GdtAccessFlags, flags: GdtFlags) -> Self {
        GdtEntry {
            limitl: 0,
            offsetl: 0,
            offsetm: 0,
            access: access.cbits(),
            flags_limith: flags.cbits() & 0xF0,
            offseth: 0
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

/// Specifies which element to load into a segment from
/// descriptor tables (i.e., is a index to LDT or GDT table
/// with some additional flags).
///
/// See Intel 3a, Section 3.4.2 "Segment Selectors"
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
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> SegmentSelector {
        SegmentSelector { bits: index << 3 | (rpl as u16) }
    }

    pub const fn from_raw(bits: u16) -> SegmentSelector {
        SegmentSelector { bits: bits }
    }
}

/// Load GDT table.
pub unsafe fn lgdt(gdt: &DescriptorTablePointer) {
    asm!("lgdt ($0)" :: "r" (gdt) : "memory");
}

pub unsafe fn set_cs(sel: SegmentSelector) {
    asm!("pushq $0; \
          leaq  1f(%rip), %rax; \
          pushq %rax; \
          lretq; \
          1:" :: "ri" (sel.bits() as usize) : "rax" "memory");
}

/// Reload stack segment register.
pub unsafe fn load_ss(sel: SegmentSelector) {
    asm!("movw $0, %ss " :: "r" (sel.bits()) : "memory");
}

/// Reload data segment register.
pub unsafe fn load_ds(sel: SegmentSelector) {
    asm!("movw $0, %ds " :: "r" (sel.bits()) : "memory");
}

/// Reload es segment register.
pub unsafe fn load_es(sel: SegmentSelector) {
    asm!("movw $0, %es " :: "r" (sel.bits()) : "memory");
}

/// Reload fs segment register.
pub unsafe fn load_fs(sel: SegmentSelector) {
    asm!("movw $0, %fs " :: "r" (sel.bits()) : "memory");
}

/// Reload gs segment register.
pub unsafe fn load_gs(sel: SegmentSelector) {
    asm!("movw $0, %gs " :: "r" (sel.bits()) : "memory");
}