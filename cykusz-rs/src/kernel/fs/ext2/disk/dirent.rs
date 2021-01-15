#![allow(dead_code)]

use crate::arch::raw::mm::VirtAddr;
use crate::kernel::fs::ext2::disk::inode::FileType;
use crate::kernel::utils::types::Align;

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

impl From<super::inode::FileType> for DirEntTypeIndicator {
    fn from(v: FileType) -> Self {
        match v {
            FileType::File => DirEntTypeIndicator::RegularFile,
            FileType::CharDev => DirEntTypeIndicator::CharDev,
            FileType::BlockDev => DirEntTypeIndicator::BlockDev,
            FileType::Dir => DirEntTypeIndicator::Directory,
            FileType::Symlink => DirEntTypeIndicator::Symlink,
            FileType::Socket => DirEntTypeIndicator::Socket,
            FileType::Fifo => DirEntTypeIndicator::Fifo,
            FileType::Unknown => DirEntTypeIndicator::Unknown,
        }
    }
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
    fn bytes(&self) -> &[u8] {
        unsafe { VirtAddr(self as *const _ as usize).as_bytes(self.ent_size() as usize) }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { VirtAddr(self as *mut _ as usize).as_bytes_mut(self.ent_size() as usize) }
    }

    pub fn inode(&self) -> u32 {
        self.inode
    }

    pub fn set_inode(&mut self, inode: u32) {
        self.inode = inode;
    }

    pub fn ent_size(&self) -> u16 {
        self.ent_size
    }

    pub fn set_ent_size(&mut self, size: u16) {
        self.ent_size = size;
    }

    pub fn name_size(&self) -> u8 {
        self.name_size
    }

    pub fn set_name_size(&mut self, size: u8) {
        self.name_size = size;
    }

    pub fn real_size(&self) -> u16 {
        if self.inode() == 0 {
            return 0;
        }

        (self.name_size as u16 + 8).align_up(4)
    }

    pub fn available_size(&self) -> u16 {
        self.ent_size() - self.real_size()
    }

    pub fn ftype(&self) -> DirEntTypeIndicator {
        self.ftype
    }

    pub fn set_ftype(&mut self, t: DirEntTypeIndicator) {
        self.ftype = t;
    }

    pub fn name(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                (self as *const _ as *const u8).offset(8),
                self.name_size() as usize,
            ))
        }
    }

    pub fn set_name(&mut self, name: &str) {
        let name_len = core::cmp::min(u8::max_value() as usize, name.len());

        let req_size = (8 + name_len).align_up(4);

        if self.ent_size() as usize >= req_size {
            self.bytes_mut()[8..8 + name_len].copy_from_slice(&name.as_bytes()[..name_len]);

            self.set_name_size(name_len as u8);
        } else {
            panic!("Failed to set inode name");
        }
    }

    pub fn extract(&mut self) -> Option<&mut DirEntry> {
        let avail = self.available_size();
        if avail >= 12 {
            self.ent_size = self.real_size();

            let new = unsafe {
                (VirtAddr(self as *const _ as usize) + self.ent_size() as usize)
                    .read_mut::<DirEntry>()
            };

            new.ent_size = avail;

            Some(new)
        } else {
            None
        }
    }
}
