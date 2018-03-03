use arch::raw::descriptor as dsc;

bitflags! {
    pub struct GdtFlags: u8 {
        const MISSING = 0;

        // Granularity bit
        const PAGE_GRANULARITY = 1 << 7;
        const LONG_MODE = 1 << 5;
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

impl GdtEntry {
    pub const MISSING: Self = Self::new(dsc::Flags::MISSING, GdtFlags::MISSING);

    pub const fn new(access: dsc::Flags, flags: GdtFlags) -> Self {
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
