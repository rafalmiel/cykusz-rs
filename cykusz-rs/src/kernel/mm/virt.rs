use crate::drivers::elf::types::ProgramFlags;
use crate::drivers::multiboot2::tags::elf::ElfSectionFlags;

bitflags! {
    pub struct PageFlags: usize {
        const WRITABLE      = 1 << 0;
        const USER          = 1 << 1;
        const NO_EXECUTE    = 1 << 2;
        const WRT_THROUGH   = 1 << 3;
        const NO_CACHE      = 1 << 4;
    }
}

impl From<crate::drivers::elf::types::ProgramFlags> for PageFlags {
    fn from(p: ProgramFlags) -> Self {
        let mut flags = PageFlags::empty();

        if p.contains(ProgramFlags::WRITABLE) {
            flags |= Self::WRITABLE;
        }

        if !p.contains(ProgramFlags::EXECUTABLE) {
            flags |= Self::NO_EXECUTE;
        }

        flags
    }
}

impl From<crate::drivers::multiboot2::elf::ElfSectionFlags> for PageFlags {
    fn from(p: ElfSectionFlags) -> Self {
        let mut flags = PageFlags::empty();

        if p.contains(ElfSectionFlags::WRITABLE) {
            flags |= Self::WRITABLE;
        }
        if !p.contains(ElfSectionFlags::EXECUTABLE) {
            flags |= Self::NO_EXECUTE;
        }

        flags
    }
}
