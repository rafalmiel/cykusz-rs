use core::mem::size_of;

use crate::kernel::mm::{MappedAddr, PhysAddr};

use super::apic::MadtHeader;
use super::hpet::HpetHeader;

#[repr(packed, C)]
pub struct AcpiStdHeader {
    pub signature: [u8; 4],
    pub length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

pub trait RsdtPtrType {
    fn as_usize(self) -> usize;
}

impl RsdtPtrType for u32 {
    fn as_usize(self) -> usize {
        self as usize
    }
}

impl RsdtPtrType for u64 {
    fn as_usize(self) -> usize {
        self as usize
    }
}

#[repr(packed, C)]
pub struct Rsdt<T: RsdtPtrType + ::core::marker::Sized> {
    pub hdr: AcpiStdHeader,
    _phantom: ::core::marker::PhantomData<T>,
}

pub struct RsdtIter<T: 'static + RsdtPtrType + ::core::marker::Sized> {
    rsdt: &'static Rsdt<T>,
    current: usize,
    total: usize,
}

impl AcpiStdHeader {
    pub fn is_valid(&self) -> bool {
        use super::util::checksum;
        unsafe { checksum(self as *const _ as *const u8, self.length as isize) }
    }
}

impl<T: RsdtPtrType> Rsdt<T> {
    pub fn entries_count(&'static self) -> usize {
        (self.hdr.length as usize - size_of::<Self>()) / core::mem::size_of::<T>()
    }

    fn raw_entries(&'static self) -> *const T {
        unsafe { (self as *const _ as *const u8).offset(size_of::<Self>() as isize) as *const T }
    }

    pub fn entry_at(&'static self, i: usize) -> &'static AcpiStdHeader {
        assert!(i < self.entries_count());

        unsafe {
            PhysAddr(
                (self.raw_entries().offset(i as isize))
                    .read_unaligned()
                    .as_usize(),
            )
            .to_mapped()
            .read_ref::<AcpiStdHeader>()
        }
    }

    pub fn entries(&'static self) -> RsdtIter<T> {
        RsdtIter::<T> {
            rsdt: self,
            current: 0,
            total: self.entries_count(),
        }
    }

    fn find_entry(&'static self, val: &[u8]) -> Option<&'static AcpiStdHeader> {
        self.entries().find(|e| &e.signature == val && e.is_valid())
    }

    pub fn find_apic_entry(&'static self) -> Option<&'static MadtHeader> {
        Some(self.find_entry(b"APIC")?.into_matd())
    }

    pub fn find_hpet_entry(&'static self) -> Option<&'static HpetHeader> {
        Some(self.find_entry(b"HPET")?.into_hpet())
    }

    pub fn find_ecdt_entry(&'static self) -> Option<&'static acpica::acpi_table_ecdt> {
        Some(self.find_entry(b"ECDT")?.into_ecdt())
    }

    pub fn find_fadt_entry(&'static self) -> Option<&'static acpica::acpi_table_fadt> {
        Some(self.find_entry(b"FACP")?.into_fadt())
    }

    pub fn find_mcfg_entry(&'static self) -> Option<&'static acpica::acpi_table_mcfg> {
        Some(self.find_entry(b"MCFG")?.into_mcfg())
    }

    pub fn print_tables(&'static self) {
        self.entries().for_each(|e| {
            println!("{}", unsafe {
                core::str::from_utf8_unchecked(&e.signature)
            });
        })
    }
}

impl<T: RsdtPtrType> Iterator for RsdtIter<T> {
    type Item = &'static AcpiStdHeader;

    fn next(&mut self) -> Option<&'static AcpiStdHeader> {
        if self.current == self.total {
            None
        } else {
            let r = Some(self.rsdt.entry_at(self.current));

            self.current += 1;

            return r;
        }
    }
}

impl AcpiStdHeader {
    unsafe fn to<T>(&'static self) -> &'static T {
        &*(self as *const _ as *const T)
    }

    pub fn into_rsdt(&'static self) -> &'static Rsdt<u32> {
        if self.is_valid() && &self.signature == b"RSDT" {
            unsafe {
                return self.to::<Rsdt<u32>>();
            }
        }
        panic!("AcpiStd: Tried to convert into invalid RSDT Header")
    }

    pub fn into_xsdt(&'static self) -> &'static Rsdt<u64> {
        if self.is_valid() && &self.signature == b"XSDT" {
            unsafe {
                return self.to::<Rsdt<u64>>();
            }
        }
        panic!("AcpiStd: Tried to convert into invalid RSDT Header")
    }

    pub fn into_matd(&'static self) -> &'static MadtHeader {
        if self.is_valid() && &self.signature == b"APIC" {
            unsafe {
                return self.to::<MadtHeader>();
            }
        }

        panic!("AcpiStd: Tried to convert into invalid MATD Header")
    }

    pub fn into_hpet(&'static self) -> &'static HpetHeader {
        if self.is_valid() && &self.signature == b"HPET" {
            unsafe {
                return self.to::<HpetHeader>();
            }
        }

        panic!("AcpiStd: Tried to convert into invalid HPET Header")
    }

    pub fn into_ecdt(&'static self) -> &'static acpica::acpi_table_ecdt {
        if self.is_valid() && &self.signature == b"ECDT" {
            unsafe {
                return self.to::<acpica::acpi_table_ecdt>();
            }
        }

        panic!("Invalid ECDT");
    }

    pub fn into_fadt(&'static self) -> &'static acpica::acpi_table_fadt {
        if self.is_valid() && &self.signature == b"FACP" {
            unsafe {
                return self.to::<acpica::acpi_table_fadt>();
            }
        }

        panic!("Invalid FADT");
    }

    pub fn into_mcfg(&'static self) -> &'static acpica::acpi_table_mcfg {
        if self.is_valid() && &self.signature == b"MCFG" {
            unsafe {
                return self.to::<acpica::acpi_table_mcfg>();
            }
        }

        panic!("Invalid MCFG")
    }
}

impl Rsdt<u32> {
    pub fn new(addr: MappedAddr) -> &'static Rsdt<u32> {
        let r = unsafe { addr.read_ref::<Rsdt<u32>>() };

        if !r.hdr.is_valid() || &r.hdr.signature != b"RSDT" {
            panic!("Rsdt addr is invalid: {}", addr);
        }

        r
    }
}

impl Rsdt<u64> {
    pub fn new(addr: MappedAddr) -> &'static Rsdt<u64> {
        let r = unsafe { addr.read_ref::<Rsdt<u64>>() };

        if !r.hdr.is_valid() || &r.hdr.signature != b"XSDT" {
            panic!("Xsdt addr is invalid: {}", addr);
        }

        r
    }
}
