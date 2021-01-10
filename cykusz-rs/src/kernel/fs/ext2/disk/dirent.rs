#![allow(dead_code)]

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DirEntTypeIndicator {
    Unknown = 0,
    RegularFile = 1,
    Directory = 2,
    CharDev = 3,
    BlockDev = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct DirEntry {
    inode: u32,
    ent_size: u16,
    name_size: u8,
    ftype: DirEntTypeIndicator,
}

impl DirEntry {
    pub fn inode(&self) -> u32 {
        self.inode
    }

    pub fn ent_size(&self) -> u16 {
        self.ent_size
    }

    pub fn name_size(&self) -> u8 {
        self.name_size
    }

    pub fn ftype(&self) -> DirEntTypeIndicator {
        self.ftype
    }

    pub fn name(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                (self as *const _ as *const u8).offset(8),
                self.name_size() as usize,
            ))
        }
    }
}
