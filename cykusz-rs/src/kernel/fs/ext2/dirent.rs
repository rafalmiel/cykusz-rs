use crate::arch::raw::mm::VirtAddr;
use crate::kernel::fs::ext2::disk::dirent::DirEntry;
use crate::kernel::fs::ext2::disk::inode::INode;
use crate::kernel::fs::ext2::Ext2Filesystem;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct DirEntIter<'a> {
    d_inode: &'a INode,
    fs: Weak<Ext2Filesystem>,
    buf: Vec<u8>,
    offset: usize,
}

impl<'a> DirEntIter<'a> {
    pub fn new(fs: Weak<Ext2Filesystem>, d_inode: &'a INode) -> DirEntIter<'a> {
        DirEntIter::<'a> {
            d_inode,
            fs,
            buf: Vec::new(),
            offset: 0,
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap().clone()
    }
}

impl<'a> Iterator for DirEntIter<'a> {
    type Item = &'a DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let fs = self.fs();
        let block_size = fs.superblock().block_size();

        if self.buf.is_empty() {
            let fs = self.fs();

            self.buf.resize(block_size, 0);

            fs.dev().read(
                self.d_inode.direct_ptr0() as usize * 2,
                self.buf.as_mut_slice(),
            );

            let ent = unsafe { VirtAddr(self.buf.as_ptr() as usize).read_ref::<DirEntry>() };

            if ent.ent_size() != 0 {
                Some(ent)
            } else {
                None
            }
        } else {
            let ent = unsafe {
                VirtAddr(self.buf.as_ptr().offset(self.offset as isize) as usize)
                    .read_ref::<DirEntry>()
            };

            self.offset += ent.ent_size() as usize;

            if self.offset >= block_size {
                return None;
            }

            let ent = unsafe {
                VirtAddr(self.buf.as_ptr().offset(self.offset as isize) as usize)
                    .read_ref::<DirEntry>()
            };

            if ent.ent_size() != 0 {
                Some(ent)
            } else {
                None
            }
        }
    }
}
