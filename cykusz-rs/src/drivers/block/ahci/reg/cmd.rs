use bit_field::BitField;
use mmio::VCell;

use crate::drivers::block::ahci::reg::FisRegH2D;
use crate::kernel::mm::PhysAddr;

bitflags! {
    pub struct HbaCmdHeaderFlags: u16 {
        const A = 1 << 5; // ATAPI
        const W = 1 << 6; // Write
        const P = 1 << 7; // Prefetchable
        const R = 1 << 8; // Reset
        const B = 1 << 9; // Bist
        const C = 1 << 10; // Clear Busy upon R_OK
    }
}

impl HbaCmdHeaderFlags {
    pub fn command_fis_length(&self) -> usize {
        self.bits().get_bits(0..=4) as usize
    }

    pub fn set_command_fis_length(&mut self, v: u8) {
        self.bits.set_bits(0..=4, v as u16);
    }

    pub fn port_multiplier(&self) -> usize {
        self.bits().get_bits(12..=15) as usize
    }

    pub fn set_port_multiplier(&mut self, m: usize) {
        self.bits.set_bits(12..=15, m as u16);
    }
}

#[repr(C, packed)]
pub struct HbaCmdHeader {
    flags: VCell<HbaCmdHeaderFlags>,

    prdtl: VCell<u16>,
    prdbc: VCell<u32>,

    ctb: VCell<PhysAddr>,

    _rsv1: [u32; 4],
}

impl HbaCmdHeader {
    pub fn flags(&self) -> HbaCmdHeaderFlags {
        unsafe { self.flags.get() }
    }

    pub fn set_flags(&mut self, f: HbaCmdHeaderFlags) {
        unsafe { self.flags.set(f) };
    }

    pub fn prdtl(&self) -> usize {
        unsafe { self.prdtl.get() as usize }
    }

    pub fn set_prdtl(&mut self, v: usize) {
        unsafe { self.prdtl.set(v as u16) };
    }

    pub fn prd_byte_count(&self) -> usize {
        unsafe { self.prdbc.get() as usize }
    }

    pub fn set_prd_byte_count(&mut self, v: usize) {
        unsafe { self.prdbc.set(v as u32) };
    }

    pub fn cmd_tbl_base_addr(&self) -> PhysAddr {
        unsafe { self.ctb.get() }
    }

    pub fn set_cmd_tbl_base_addr(&mut self, a: PhysAddr) {
        unsafe { self.ctb.set(a) }
    }

    pub fn cmd_tbl(&self) -> &mut HbaCmdTbl {
        unsafe { self.cmd_tbl_base_addr().to_virt().read_mut::<HbaCmdTbl>() }
    }
}

#[repr(C, packed)]
pub struct HbaCmdTbl {
    cfis: [u8; 64],

    acmd: [u8; 16],

    _rsv1: [u8; 48],

    prdt_entry: [HbaPrdtEntry; 1],
}

impl HbaCmdTbl {
    pub fn cfis_as_h2d_mut(&mut self) -> &mut FisRegH2D {
        unsafe { &mut *(self.cfis.as_mut_ptr() as *mut FisRegH2D) }
    }

    pub fn prdt_entry_mut(&mut self, i: usize) -> &mut HbaPrdtEntry {
        unsafe { &mut *self.prdt_entry.as_mut_ptr().offset(i as isize) }
    }
}

#[repr(C, packed)]
pub struct HbaPrdtEntry {
    dba: VCell<PhysAddr>,

    _rsv1: u32,

    flags: VCell<u32>,
}

impl HbaPrdtEntry {
    pub fn database_address(&self) -> PhysAddr {
        unsafe { self.dba.get() }
    }

    pub fn set_database_address(&mut self, addr: PhysAddr) {
        unsafe {
            self.dba.set(addr);
        }
    }

    pub fn data_byte_count(&self) -> usize {
        unsafe { self.flags.get().get_bits(0..=21) as usize }
    }

    pub fn set_data_byte_count(&mut self, b: usize) {
        unsafe {
            self.flags.set(*self.flags.get().set_bits(0..=21, b as u32));
        }
    }

    pub fn interrupt_on_completion(&self) -> bool {
        unsafe { self.flags.get().get_bit(31) }
    }

    pub fn set_interrupt_on_completion(&mut self, i: bool) {
        unsafe { self.flags.set(*self.flags.get().set_bit(31, i)) }
    }
}
