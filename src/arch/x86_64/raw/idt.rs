use arch::raw::segmentation::SegmentSelector;
use arch::raw::descriptor as dsc;

#[derive(Copy, Clone)]
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
            offset_1: (ptr & 0xF) as u16,
            selector,
            ist: 0,
            type_attr: flags,
            offset_2: ((ptr >> 16) & 0xF)  as u16,
            offset_3: ((ptr >> 32) & 0xFF) as u32,
            zero: 0,
        }
    }
}
