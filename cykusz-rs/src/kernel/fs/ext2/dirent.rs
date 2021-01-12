use alloc::sync::Arc;
use alloc::vec::Vec;

use core::marker::PhantomData;

use crate::arch::raw::mm::VirtAddr;
use crate::kernel::fs::ext2::disk::dirent::DirEntry;
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::ext2::reader::INodeReader;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::Spin;

pub struct SysDirEntIter<'a> {
    iter: Spin<DirEntIter<'a>>,
}

impl<'a> SysDirEntIter<'a> {
    pub fn new(inode: Arc<LockedExt2INode>) -> SysDirEntIter<'a> {
        SysDirEntIter::<'a> {
            iter: Spin::new(DirEntIter::new(inode)),
        }
    }
}

impl<'a> crate::kernel::fs::vfs::DirEntIter for SysDirEntIter<'a> {
    fn next(&self) -> Option<crate::kernel::fs::vfs::DirEntry> {
        let mut lock = self.iter.lock();
        if let Some(e) = lock.next() {
            Some(lock.inode.mk_dirent(e))
        } else {
            None
        }
    }
}

pub struct DirEntIter<'a> {
    inode: Arc<LockedExt2INode>,
    reader: INodeReader,
    buf: Vec<u8>,
    offset: usize,
    block: usize,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> DirEntIter<'a> {
    pub fn new(inode: Arc<LockedExt2INode>) -> DirEntIter<'a> {
        DirEntIter::<'a> {
            inode: inode.clone(),
            reader: INodeReader::new(inode, 0),
            buf: Vec::new(),
            offset: 0,
            block: 0,
            _phantom: PhantomData::default(),
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.inode.fs()
    }
}

impl<'a> Iterator for DirEntIter<'a> {
    type Item = &'a DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let fs = self.fs();
        let block_size = fs.superblock().block_size();

        let file_size = { self.inode.read().d_inode().size_lower() };

        let ent = loop {
            if self.offset >= file_size as usize {
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
