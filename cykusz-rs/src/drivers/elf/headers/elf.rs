use crate::drivers::elf::types;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ElfHeader {
    pub ei_magic: [u8; 4],
    pub ei_class: types::Class,
    pub ei_data: types::Endianess,
    pub ei_version: u8,
    pub ei_osabi: types::Abi,
    pub _pad: u64,
    pub e_type: types::BinType,
    pub e_machine: types::Machine,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}
