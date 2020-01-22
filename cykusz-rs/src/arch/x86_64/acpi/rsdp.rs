use core::mem::size_of;

use crate::arch::acpi;
use crate::arch::mm::*;

pub enum Address {
    Rsdp(MappedAddr),
    Xsdp(MappedAddr),
}

trait Header {
    fn signature(&'static self) -> &'static [u8; 8];
    fn rsdt_address(&'static self) -> MappedAddr;
    fn revision(&'static self) -> u8;
}

#[repr(packed, C)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
}

#[repr(packed, C)]
struct Rsdp20 {
    rsdp: Rsdp,
    // Ver. 2.0 fields:
    length: u32,
    xsdt_address: u64,
    pub extended_checksum: u8,
    reserved: [u8; 3],
}

fn is_valid<T: Header>(f: &'static T) -> bool {
    unsafe {
        if f.signature() as &[u8] != b"RSD PTR " {
            false
        } else {
            acpi::util::checksum(f as *const _ as *const u8, size_of::<T>() as isize)
        }
    }
}

impl Header for Rsdp {
    fn signature(&'static self) -> &'static [u8; 8] {
        return &self.signature;
    }

    fn rsdt_address(&'static self) -> MappedAddr {
        PhysAddr(self.rsdt_address as usize).to_mapped()
    }

    fn revision(&'static self) -> u8 {
        self.revision
    }
}

impl Header for Rsdp20 {
    fn signature(&'static self) -> &'static [u8; 8] {
        return &self.rsdp.signature;
    }
    fn rsdt_address(&'static self) -> MappedAddr {
        PhysAddr(self.xsdt_address as usize).to_mapped()
    }
    fn revision(&'static self) -> u8 {
        self.rsdp.revision
    }
}

fn find_hdr<T: Header>() -> Option<&'static impl Header> {
    let ebda_address = unsafe {
        PhysAddr((PhysAddr(0x40E as usize).to_mapped().read::<u16>()) as usize * 4).to_mapped()
    };

    let ebda_iter = (ebda_address..(ebda_address + 1024)).step_by(0x10);

    for addr in ebda_iter {
        let ptr = unsafe { addr.read_ref::<T>() };

        if is_valid(ptr) {
            return Some(ptr);
        }
    }

    let iter = (PhysAddr(0xE0_000 as usize).to_mapped()..PhysAddr(0x100_000 as usize).to_mapped())
        .step_by(0x10);

    for addr in iter {
        let ptr = unsafe { addr.read_ref::<T>() };

        if is_valid(ptr) {
            return Some(ptr);
        }
    }

    None
}

pub fn find_rsdt_address() -> Option<Address> {
    // Look for Rsdp20 header, and if it does not exists, look for Rsdp
    let rsdp20 = find_hdr::<Rsdp20>();
    if let Some(x) = rsdp20 {
        if x.rsdt_address().to_phys().0 != 0 {
            return Some(Address::Xsdp(x.rsdt_address()));
        }
    }
    return Some(Address::Rsdp(find_hdr::<Rsdp>()?.rsdt_address()));
}
