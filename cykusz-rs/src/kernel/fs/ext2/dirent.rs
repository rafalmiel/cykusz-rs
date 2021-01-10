use crate::arch::raw::mm::VirtAddr;
use crate::kernel::fs::ext2::disk::dirent::DirEntry;
use crate::kernel::fs::ext2::disk::inode::INode;
use crate::kernel::fs::ext2::reader::INodeReader;
use crate::kernel::fs::ext2::Ext2Filesystem;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct DirEntIter<'a> {
    d_inode: &'a INode,
    fs: Weak<Ext2Filesystem>,
    reader: INodeReader<'a>,
    buf: Vec<u8>,
    offset: usize,
    block: usize,
}

impl<'a> DirEntIter<'a> {
    pub fn new(fs: Weak<Ext2Filesystem>, d_inode: &'a INode) -> DirEntIter<'a> {
        DirEntIter::<'a> {
            d_inode,
            fs: fs.clone(),
            reader: INodeReader::new(d_inode, fs, 0),
            buf: Vec::new(),
            offset: 0,
            block: 0,
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

        let ent = loop {
            if self.offset >= self.d_inode.size_lower() as usize {
                return None;
            }

            let block = self.offset / block_size;

            if self.buf.is_empty() || block > self.block {
                self.buf.resize(block_size, 0);
                if self.reader.read(self.buf.as_mut_slice()) == 0 {
                    return None;
                }
                self.block = block;
            }

            let ent = unsafe {
                VirtAddr(
                    self.buf
                        .as_ptr()
                        .offset(self.offset as isize % block_size as isize)
                        as usize,
                )
                .read_ref::<DirEntry>()
            };

            self.offset += ent.ent_size() as usize;

            if ent.inode() != 0 {
                break ent;
            }
        };

        if ent.ent_size() != 0 {
            Some(ent)
        } else {
            None
        }
    }
}
