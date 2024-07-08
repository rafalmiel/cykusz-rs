use crate::arch::raw::descriptor as dsc;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct GdtFlags: u8 {
        const MISSING = 0;

        // Granularity bit
        const PAGE_GRANULARITY = 1 << 7;
        const LONG_MODE = 1 << 5;
    }
}

impl GdtFlags {
    pub const fn cbits(&self) -> u8 {
        self.bits()
    }
}

#[repr(C, packed)]
pub struct GdtEntry {
    pub limitl: u16,
    pub offsetl: u16,
    pub offsetm: u8,
    pub access: u8,
    pub flags_limith: u8,
    pub offseth: u8,
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
            offseth: 0,
        }
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.offsetl = offset as u16;
        self.offsetm = (offset >> 16) as u8;
        self.offseth = (offset >> 24) as u8;
    }

    pub fn set_limit(&mut self, limit: u32) {
        self.limitl = limit as u16;
        self.flags_limith = self.flags_limith & 0xF0 | ((limit >> 16) as u8) & 0x0F;
    }

    pub fn set_raw(&mut self, val: u64) {
        unsafe {
            *(self as *mut _ as *mut u64) = val;
        }
    }
}
