#![allow(dead_code)]

use bit_field::BitField;

use crate::kernel::mm::heap::{allocate_align, deallocate_align};
use crate::kernel::mm::VirtAddr;

pub struct Mbr {
    data: VirtAddr,
}

pub struct Partition<'a> {
    data: &'a [u8],
}

impl Mbr {
    pub fn new() -> Mbr {
        let mem = allocate_align(512, 0x1000).unwrap();

        let mbr = Mbr {
            data: VirtAddr(mem as usize),
        };

        mbr
    }

    pub fn bytes(&self) -> &[u8] {
        unsafe { self.data.as_bytes(512) }
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { self.data.as_bytes_mut(512) }
    }

    pub fn is_valid(&self) -> bool {
        self.bytes()[510..] == [0x55, 0xAA]
    }

    pub fn partition(&self, idx: usize) -> Option<Partition> {
        if idx >= 4 {
            return None;
        }
        let off = 0x01BEusize + 0x10 * idx;

        Some(Partition {
            data: &self.bytes()[off..off + 16],
        })
    }
}

impl Partition<'_> {
    pub fn flags(&self) -> u8 {
        self.data[0]
    }

    pub fn starting_head(&self) -> u8 {
        self.data[1]
    }

    pub fn starting_sector(&self) -> u8 {
        self.data[2].get_bits(0..6)
    }

    pub fn starting_cylinder(&self) -> u16 {
        ((self.data[2].get_bits(6..8) as u16) << 8) | self.data[3] as u16
    }

    pub fn system_id(&self) -> u8 {
        self.data[4]
    }

    pub fn ending_head(&self) -> u8 {
        self.data[5]
    }

    pub fn ending_sector(&self) -> u8 {
        self.data[6].get_bits(0..6)
    }

    pub fn ending_cylinder(&self) -> u16 {
        ((self.data[6].get_bits(6..8) as u16) << 8) | self.data[7] as u16
    }

    pub fn relative_sector(&self) -> usize {
        unsafe { *(self.data.as_ptr().offset(8) as *const u32) as usize }
    }

    pub fn total_sectors(&self) -> usize {
        unsafe { *(self.data.as_ptr().offset(12) as *const u32) as usize }
    }
}

impl Drop for Mbr {
    fn drop(&mut self) {
        deallocate_align(self.data.0 as *mut u8, 512, 0x1000);
    }
}
