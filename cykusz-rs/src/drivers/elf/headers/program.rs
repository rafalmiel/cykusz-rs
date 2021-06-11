use crate::drivers::elf::types;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ProgramHeader {
    pub p_type: types::ProgramType,
    pub p_flags: types::ProgramFlags,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}
