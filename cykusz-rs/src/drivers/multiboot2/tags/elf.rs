use crate::drivers::multiboot2::tags::Tag;
use crate::kernel::mm::{PhysAddr, VirtAddr};

#[repr(C)]
pub struct Elf {
    pub tag: Tag,
    pub num: u32,
    pub entsize: u32,
    pub shndx: u32,
    pub first_entry: [u8; 0],
}

#[repr(C)]
pub struct ElfSection {
    name: u32,
    pub typ: u32,
    pub flags: ElfSectionFlags,
    addr: u64,
    offset: u64,
    pub size: u64,
    link: u32,
    info: u32,
    addr_align: u64,
    entsize: u64,
}

pub struct ElfSectionIter {
    current: *const ElfSection,
    remaining: u32,
    entry_size: u32,
}

#[allow(unused)]
#[repr(u32)]
pub enum ElfSectionType {
    Unused = 0,
    ProgramSection = 1,
    LinkerSymbolTable = 2,
    RelaRelocation = 4,
    SymbolHashTable = 5,
    DynamicLinkingTable = 6,
    Note = 7,
    Uninitialized = 8,
    RelRelocation = 9,
    Reserved = 10,
    DynamicLoaderSymbolTable = 11,
}

bitflags!(
    pub struct ElfSectionFlags: u64 {
        const WRITABLE = 1;
        const ALLOCATED = 2;
        const EXECUTABLE = 4;
    }
);

impl Elf {
    pub fn sections(&'static self) -> ElfSectionIter {
        ElfSectionIter {
            current: self.first_entry.as_ptr() as *const ElfSection,
            remaining: self.num - 1,
            entry_size: self.entsize,
        }
    }
}

impl ElfSection {
    pub fn address(&self) -> VirtAddr {
        PhysAddr(self.addr as usize).to_virt()
    }

    pub fn end_address(&self) -> VirtAddr {
        PhysAddr(self.addr as usize + self.size as usize).to_virt()
    }
}

impl Iterator for ElfSectionIter {
    type Item = ElfSection;

    fn next(&mut self) -> Option<ElfSection> {
        if self.remaining == 0 {
            None
        } else {
            let section = unsafe {
                self.current.read_unaligned()
            };

            self.current = (self.current as *const _ as u64 + self.entry_size as u64)
                as *const ElfSection;

            self.remaining -= 1;

            if (section.typ == ElfSectionType::Unused as u32)
                || !section.flags.contains(ElfSectionFlags::ALLOCATED)
            {
                self.next()
            } else {
                Some(section)
            }
        }
    }
}
