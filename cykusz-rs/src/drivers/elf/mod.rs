pub use headers::elf::ElfHeader;
pub use headers::program::ProgramHeader;

use crate::kernel::mm::MappedAddr;

pub mod headers;
pub mod types;

impl ElfHeader {
    pub unsafe fn load(exe: &[u8]) -> Option<&ElfHeader> {
        let hdr = &*(exe.as_ptr() as *const ElfHeader);

        if !hdr.is_valid() {
            return None;
        }

        Some(hdr)
    }

    pub fn is_valid(&self) -> bool {
        &self.ei_magic == b"\x7FELF"
    }

    pub fn programs(&self) -> ProgramIter {
        ProgramIter {
            current: MappedAddr(self as *const _ as usize) + self.e_phoff as usize,
            count: self.e_phnum as usize,
        }
    }
}

pub struct ProgramIter {
    current: MappedAddr,
    count: usize,
}

impl Iterator for ProgramIter {
    type Item = &'static ProgramHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        let hdr = unsafe { self.current.read_ref::<ProgramHeader>() };

        self.current += core::mem::size_of::<ProgramHeader>();
        self.count -= 1;

        return Some(hdr);
    }
}
