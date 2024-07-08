use core::iter::FilterMap;

use crate::arch::acpi::rsdt::AcpiStdHeader;
use crate::kernel::mm::{MappedAddr, PhysAddr};

bitflags! {
    #[derive(Copy, Clone, PartialEq)]
    pub struct MatdEntryType : u8{
        const PROC_LOCAL_APIC = 0x0;
        const PROC_IO_APIC = 0x1;
        const INT_SRC_OVERRIDE = 0x2;
    }
}

#[repr(packed, C)]
pub struct MadtHeader {
    hdr: AcpiStdHeader,
    pub local_controller_address: u32,
    flags: u32,
}

#[repr(packed, C)]
pub struct MadtEntry {
    typ: MatdEntryType,
    length: u8,
}

pub struct MadtIter {
    current: *const u8,
    limit: *const u8,
}

#[repr(packed, C)]
pub struct MadtEntryIntSrc {
    matd: MadtEntry,
    bus_src: u8,
    irq_src: u8,
    global_sys_int: u32,
    flags: u16,
}

impl MadtEntryIntSrc {
    pub fn irq_src(&'static self) -> u8 {
        self.irq_src
    }

    pub fn global_sys_int(&'static self) -> u32 {
        self.global_sys_int
    }

    pub fn level_triggered(&'static self) -> bool {
        self.flags & 8 == 8
    }

    pub fn active_low(&'static self) -> bool {
        self.flags & 2 == 2
    }
}

#[repr(packed, C)]
pub struct MadtEntryLocalApic {
    matd: MadtEntry,
    pub proc_id: u8,
    pub apic_id: u8,
    flags: u32,
}

#[repr(packed, C)]
pub struct MadtEntryIOApic {
    matd: MadtEntry,
    pub ioapic_id: u8,
    reserved: u8,
    pub ioapic_address: u32,
    pub global_int_base: u32,
}

impl MadtEntryIOApic {
    pub fn ioapic_address(&'static self) -> MappedAddr {
        PhysAddr(self.ioapic_address as usize).to_mapped()
    }
}

impl MadtEntryLocalApic {
    pub fn proc_is_enabled(&self) -> bool {
        self.flags == 1
    }
}

impl MadtHeader {
    pub fn entries(&'static self) -> MadtIter {
        MadtIter {
            current: unsafe {
                (self as *const _ as *const u8)
                    .offset(::core::mem::size_of::<MadtHeader>() as isize)
            },
            limit: unsafe { (self as *const _ as *const u8).offset(self.hdr.length as isize) },
        }
    }

    pub fn lapic_address(&'static self) -> MappedAddr {
        PhysAddr(self.local_controller_address as usize).to_mapped()
    }

    pub fn lapic_entries(
        &'static self,
    ) -> FilterMap<MadtIter, fn(&MadtEntry) -> Option<&'static MadtEntryLocalApic>> {
        self.entries().filter_map(|e| {
            if e.typ == MatdEntryType::PROC_LOCAL_APIC {
                unsafe { Some(&*(e as *const _ as *const MadtEntryLocalApic)) }
            } else {
                None
            }
        })
    }

    pub fn ioapic_entries(
        &'static self,
    ) -> FilterMap<MadtIter, fn(&MadtEntry) -> Option<&'static MadtEntryIOApic>> {
        self.entries().filter_map(|e| {
            if e.typ == MatdEntryType::PROC_IO_APIC {
                unsafe { Some(&*(e as *const _ as *const MadtEntryIOApic)) }
            } else {
                None
            }
        })
    }

    pub fn intsrc_entries(
        &'static self,
    ) -> FilterMap<MadtIter, fn(&MadtEntry) -> Option<&'static MadtEntryIntSrc>> {
        self.entries().filter_map(|e| {
            if e.typ == MatdEntryType::INT_SRC_OVERRIDE {
                unsafe { Some(&*(e as *const _ as *const MadtEntryIntSrc)) }
            } else {
                None
            }
        })
    }

    pub fn find_irq_remap(&'static self, int: u32) -> Option<&'static MadtEntryIntSrc> {
        self.intsrc_entries()
            .find(|i| i.irq_src() as u32 == int)
            .map_or(None, |e| Some(e))
    }
}

impl Iterator for MadtIter {
    type Item = &'static MadtEntry;

    fn next(&mut self) -> Option<&'static MadtEntry> {
        if self.current < self.limit {
            let r = unsafe { &*(self.current as *const MadtEntry) };

            unsafe {
                self.current = self.current.offset(r.length as isize);
            };

            return Some(r);
        }

        return None;
    }
}
