use alloc::sync::Arc;
use core::marker::PhantomData;

use crate::arch::raw::mm::VirtAddr;
use crate::kernel::fs::ext2::buf_block::BufBlock;
use crate::kernel::fs::ext2::disk::dirent::{DirEntTypeIndicator, DirEntry};
use crate::kernel::fs::ext2::disk::inode::FileType;
use crate::kernel::fs::ext2::idata::INodeData;
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::Spin;
use crate::kernel::utils::types::Align;

pub struct SysDirEntIter<'a> {
    parent: Arc<crate::kernel::fs::dirent::DirEntry>,
    iter: Spin<DirEntIter<'a>>,
}

impl<'a> SysDirEntIter<'a> {
    pub fn new(
        parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        inode: Arc<LockedExt2INode>,
    ) -> SysDirEntIter<'a> {
        SysDirEntIter::<'a> {
            parent,
            iter: Spin::new(DirEntIter::new(inode)),
        }
    }
}

impl<'a> crate::kernel::fs::vfs::DirEntIter for SysDirEntIter<'a> {
    fn next(&self) -> Option<Arc<crate::kernel::fs::dirent::DirEntry>> {
        let mut lock = self.iter.lock();
        if let Some(e) = lock.next() {
            Some(lock.inode.mk_dirent(self.parent.clone(), &e))
        } else {
            None
        }
    }
}

pub struct DirEntIter<'a> {
    inode: Arc<LockedExt2INode>,
    reader: INodeData,
    buf: BufBlock,
    offset: usize,
    block: usize,
    skip_empty: bool,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> DirEntIter<'a> {
    pub fn new(inode: Arc<LockedExt2INode>) -> DirEntIter<'a> {
        DirEntIter::<'a> {
            inode: inode.clone(),
            reader: INodeData::new(inode, 0),
            buf: BufBlock::empty(),
            offset: 0,
            block: 0,
            skip_empty: true,
            _phantom: PhantomData::default(),
        }
    }

    pub fn new_no_skip(inode: Arc<LockedExt2INode>) -> DirEntIter<'a> {
        let mut iter = Self::new(inode);
        iter.skip_empty = false;

        iter
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.inode.fs()
    }
    pub fn remove_dir_entry(&mut self, name: &str) -> Result<()> {
        if let Some(e) = self.find(|e| e.name() == name) {
            self.fs().get_inode(e.inode() as usize).unref_hardlink();

            let fs = self.fs();

            let block_size = fs.superblock().block_size();

            e.set_inode(0);
            e.set_name_size(0);
            e.set_ftype(DirEntTypeIndicator::Unknown);

            let mut entry_at = |o: usize| unsafe {
                VirtAddr(self.buf.slice_mut().as_mut_ptr().offset(o as isize) as usize)
                    .read_mut::<DirEntry>()
            };

            let mut offset = 0;

            let mut ent = entry_at(offset);

            offset += ent.ent_size() as usize;

            while offset < block_size {
                let next = entry_at(offset);

                if next.inode() == 0 {
                    let nsize = next.ent_size() as usize;

                    ent.set_ent_size(ent.ent_size() + nsize as u16);

                    offset += nsize;
                } else {
                    ent = next;

                    offset += ent.ent_size() as usize;
                }
            }

            self.sync_current_buf();

            return Ok(());
        }

        Err(FsError::EntryNotFound)
    }

    pub fn add_dir_entry(
        &mut self,
        target: &LockedExt2INode,
        typ: FileType,
        name: &str,
    ) -> Result<()> {
        let fs = self.fs();

        let required_size = (name.len() + 8).align_up(4);

        if let Some(found) = self.find(|el| el.available_size() as usize >= required_size) {
            let target_id = target.id()?;
            if let Some(entry) = found.extract() {
                entry.set_inode(target_id as u32);
                entry.set_ftype(typ.into());
                entry.set_name(name);

                {
                    let mut inner = target.d_inode_writer();

                    inner.inc_hl_count();
                }

                self.sync_current_buf();

                self.offset -= entry.ent_size() as usize;
            } else {
                panic!("Failed to extract DirEnt");
            }

            if typ == FileType::Dir && ![".", ".."].contains(&name) {
                fs.group_descs().inc_dir_count(target_id);
            }

            Ok(())
        } else {
            let file_size = { self.inode.read().d_inode().size_lower() } as usize;

            if self.offset >= file_size {
                return if let Some(new_block) =
                    self.reader.append_block(fs.superblock().block_size())
                {
                    let entry = unsafe {
                        VirtAddr(new_block.bytes().as_ptr() as usize).read_mut::<DirEntry>()
                    };
                    entry.set_ent_size(new_block.len() as u16);
                    entry.set_inode(0);

                    fs.write_block(new_block.block(), new_block.bytes());

                    self.add_dir_entry(target, typ, name)
                } else {
                    Err(FsError::NotSupported)
                };
            } else {
                println!(
                    "Unreachable? offset {} filesize {} inode: {}",
                    self.offset,
                    file_size,
                    self.inode.id().unwrap()
                );
                unreachable!();
            }
        }
    }

    pub fn sync_current_buf(&self) {
        if !self.buf.is_empty() {
            self.fs()
                .write_block(self.buf.block(), self.buf.bytes())
                .expect("Failed to sync BufBlock");
        }
    }
}

impl<'a> Iterator for DirEntIter<'a> {
    type Item = &'a mut super::disk::dirent::DirEntry;

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
                self.block = block;
                if let Some(b) = self.reader.read_next_block() {
                    self.buf = b;
                } else {
                    return None;
                }
            }

            let ent = unsafe {
                VirtAddr(
                    self.buf
                        .bytes()
                        .as_ptr()
                        .offset(self.offset as isize % block_size as isize)
                        as usize,
                )
                .read_mut::<DirEntry>()
            };

            self.offset += ent.ent_size() as usize;

            if !self.skip_empty || ent.inode() != 0 {
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
