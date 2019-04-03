use crate::arch::acpi::rsdt::AcpiStdHeader;

#[repr(packed, C)]
pub struct HpetHeader {
    pub hdr: AcpiStdHeader,
    pub hardware_rev_id: u8,
    pub flags: u8,
    pub pci_vendor_id: u16,
    pub addr_space_id: u8,
    pub addr_register_bit_width: u8,
    pub addr_register_bit_offset: u8,
    pub _reserved: u8,
    pub address: u64,
    pub hpet_number: u8,
    pub minimum_tick: u16,
    pub page_protection: u8,
}

impl HpetHeader {}
