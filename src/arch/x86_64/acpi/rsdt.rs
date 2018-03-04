use core::mem::size_of;

use kernel::mm::{MappedAddr, PhysAddr};

const MATD_ENTRY_PROC_LOCAL_APIC: u8  = 0x0;
const MATD_ENTRY_PROC_IOAPIC: u8      = 0x1;
const MATD_ENTRY_INT_SRC_OVERRIDE: u8 =	0x2;

#[repr(packed, C)]
struct RSDTHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32
}

#[repr(packed, C)]
struct MATDHeader {
	rsdt: RSDTHeader,
	local_controller_address: u32,
	flags: u32
}

#[repr(packed, C)]
struct MATDEntry {
	typ: u8,
	length: u8
}

#[repr(packed, C)]
struct MATDEntryLocalApic {
    matd: MATDEntry,
	proc_id: u8,
	apic_id: u8,
	flags: u32
}

#[repr(packed, C)]
struct MATDEntryIOApic {
    matd: MATDEntry,
	ioapic_id: u8,
	reserved: u8,
	ioapic_address: u32,
	global_int_base: u32
}

#[repr(packed, C)]
struct MATDEntryIntSrc {
    matd: MATDEntry,
	bus_src: u8,
	irq_src: u8,
	global_sys_int: u32,
	flags: u16
}

pub struct Rsdt {
    matd: Option<&'static MATDHeader>,
    ioapic_address: Option<PhysAddr>
}

impl Rsdt {
    pub const fn new() -> Rsdt {
        Rsdt {
            matd: None,
            ioapic_address: None
        }
    }

    fn parse_matd(&mut self, matd_header: &'static MATDHeader) {
        self.matd = Some(matd_header);

        unsafe {
            let mut a = matd_header as *const _ as *const u8;
            let limit: *const u8 = a.offset(matd_header.rsdt.length as isize);
            a = a.offset(size_of::<MATDHeader>() as isize);

            while a < limit {
                let entry = &*(a as *const MATDEntry);

                match entry.typ {
                    MATD_ENTRY_PROC_LOCAL_APIC => {
                        let localapic = &*(a as *const MATDEntryLocalApic);
                        println!("[ INFO ] Local APIC: procid: {}, apicid: {}, flags: 0x{:x}",
                            localapic.proc_id, localapic.apic_id, localapic.flags);
                    },
                    MATD_ENTRY_PROC_IOAPIC => {
                        let ioapic = &*(a as *const MATDEntryIOApic);
                        self.ioapic_address = Some(PhysAddr(ioapic.ioapic_address as usize));

                        //println!("IOApic: address: {}", self.ioapic_address.unwrap());
                    },
                    _ => {}
                }

                a = a.offset(entry.length as isize);
            }
        }
    }

    pub fn remap_irq(&self, irq: u32) -> Option<u32> {
        self.matd.and_then(|matd| {
            unsafe {
                let mut a = matd as *const _ as *const u8;
                let limit: *const u8 = a.offset(matd.rsdt.length as isize);
                a = a.offset(size_of::<MATDHeader>() as isize);

                while a < limit {
                    let entry = &*(a as *const MATDEntry);

                    match entry.typ {
                        MATD_ENTRY_INT_SRC_OVERRIDE => {
                            let isrc = &*(a as *const MATDEntryIntSrc);
                            if isrc.irq_src as u32 == irq {
                                return Some(isrc.global_sys_int);
                            }
                        },
                        _ => {}
                    }

                    a = a.offset(entry.length as isize);
                }

                Some(irq)
            }
        })
    }

    pub fn local_controller_address(&self) -> Option<MappedAddr> {
        self.matd.and_then(|addr| {
            Some(PhysAddr(addr.local_controller_address as usize).to_mapped())
        })
    }

    pub fn ioapic_address(&self) -> Option<MappedAddr> {
        self.ioapic_address.and_then(|addr| {
            Some(addr.to_mapped())
        })
    }

    pub fn init(&mut self, rsdt_address: MappedAddr) {
        use super::util::checksum;
        unsafe {
            let rsdt_header = &*(rsdt_address.0 as *const RSDTHeader);

            if &rsdt_header.signature == b"RSDT"
               && checksum(rsdt_header as *const _ as *const u8,
                           rsdt_header.length as isize) {

                let entries = (rsdt_header.length - size_of::<RSDTHeader>() as u32) / 4;

                for i in 0..entries {
                    let entry = *((rsdt_header as *const _ as usize
                                   + size_of::<RSDTHeader>() + i as usize * 4) as *const u32);

                    let hdr = &*(PhysAddr(entry as usize).to_mapped().0 as *const RSDTHeader);

                    if &hdr.signature == b"APIC"
                       && checksum(hdr as *const _ as *const u8, hdr.length as isize) {
                        self.parse_matd(&*(hdr as *const _ as *const MATDHeader));
                    }
                }
            }
        }
    }
}
