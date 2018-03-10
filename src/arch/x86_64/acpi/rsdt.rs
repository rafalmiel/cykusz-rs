use core::mem::size_of;

use kernel::mm::{MappedAddr, PhysAddr};

use super::apic::Matd;

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
    creator_revision: u32
}

#[repr(packed, C)]
pub struct Rsdt {
    pub hdr: AcpiStdHeader,
}

pub struct RsdtIter {
    rsdt: &'static Rsdt,
    current: usize,
    total: usize,
}

impl AcpiStdHeader {
    pub fn is_valid(&self) -> bool {
        use super::util::checksum;
        unsafe {
            checksum(self as *const _ as *const u8, self.length as isize)
        }

    }
}

impl Rsdt {
    pub fn new(addr: MappedAddr) -> &'static Rsdt {
        let r = unsafe {
            addr.read_ref::<Rsdt>()
        };

        if !r.hdr.is_valid() || &r.hdr.signature != b"RSDT" {
            panic!("Rsdt addr is invalid: {}", addr);
        }

        r
    }

    pub fn entries_count(&'static self) -> usize {
        (self.hdr.length as usize - size_of::<Self>()) / 4
    }

    fn raw_entries(&'static self) -> *const u32 {
        unsafe {
            (self as *const _ as *const u8).offset(size_of::<Self>() as isize) as *const u32
        }
    }

    pub fn entry_at(&'static self, i: usize) -> &'static AcpiStdHeader {
        assert!(i < self.entries_count());

        unsafe {
            PhysAddr(
                (self.raw_entries().offset(i as isize)).read() as usize
            ).to_mapped().read_ref::<AcpiStdHeader>()
        }
    }

    pub fn entries(&'static self) -> RsdtIter {
        RsdtIter {
            rsdt: self,
            current: 0,
            total: self.entries_count()
        }
    }

    fn find_entry(&'static self, val: &[u8]) -> Option<&'static AcpiStdHeader> {
        self.entries().find(|e| {
            &e.signature == val && e.is_valid()
        })
    }

    pub fn find_apic_entry(&'static self) -> Option<&'static Matd> {
        Some(self.find_entry(b"APIC")?.into_matd())
    }

}

impl Iterator for RsdtIter {
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

    pub fn into_rsdt(&'static self) -> &'static Rsdt {
        if self.is_valid() &&
            &self.signature == b"RSDT" {

            unsafe {
                return self.to::<Rsdt>();
            }
        }
        panic!("AcpiStd: Tried to convert into invalid RSDT Header")
    }

    pub fn into_matd(&'static self) -> &'static Matd {
        if self.is_valid() &&
            &self.signature == b"APIC" {

            unsafe {
                return self.to::<Matd>();
            }
        }

        panic!("AcpiStd: Tried to convert into invalid MATD Header")
    }
}
