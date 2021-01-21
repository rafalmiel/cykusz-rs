use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::ops::{Deref, DerefMut};

use syscall_defs::FileType;

use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::ext2::dirent::{DirEntIter, SysDirEntIter};
use crate::kernel::fs::ext2::disk;
use crate::kernel::fs::ext2::idata::INodeData;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Metadata;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::{RwSpin, RwSpinReadGuard, RwSpinWriteGuard};
use crate::kernel::utils::slice::ToBytes;

pub struct LockedExt2INode {
    node: RwSpin<Ext2INode>,
    fs: Weak<Ext2Filesystem>,
    self_ref: Weak<LockedExt2INode>,
}

impl LockedExt2INode {
    pub fn new(fs: Weak<Ext2Filesystem>, id: usize) -> Arc<LockedExt2INode> {
        let ptr = Arc::new_cyclic(|me| LockedExt2INode {
            node: RwSpin::new(Ext2INode::new(fs.clone(), id)),
            fs,
            self_ref: me.clone(),
        });

        ptr
    }

    pub fn mk_dirent(&self, parent: Arc<DirEntry>, de: &disk::dirent::DirEntry) -> Arc<DirEntry> {
        let inode = self.fs().get_inode(de.inode() as usize);

        DirEntry::new(parent, inode, String::from(de.name()))
    }

    pub fn mk_inode(&self, typ: FileType) -> Result<Arc<LockedExt2INode>> {
        let fs = self.fs();

        let parent_id = self.id()?;

        if let Some(new) = fs.alloc_inode(parent_id) {
            let mut inner = new.d_inode_writer();

            *inner = disk::inode::INode::default();

            inner.set_ftype(typ.into());
            inner.set_perm(0o755);
            inner.set_user_id(0);
            inner.set_group_id(0);

            let time = crate::kernel::time::unix_timestamp();
            inner.set_creation_time(time as u32);
            inner.set_last_modification(time as u32);
            inner.set_last_access(time as u32);

            drop(inner);

            let result: Result<()> = try {
                if typ == FileType::Dir {
                    let mut iter = DirEntIter::new_no_skip(new.clone());
                    iter.add_dir_entry(&new, disk::inode::FileType::Dir, ".")?;
                    iter.add_dir_entry(&self, disk::inode::FileType::Dir, "..")?;
                }

                ()
            };

            if result.is_err() {
                fs.free_inode(&new);
            } else {
                return Ok(new);
            }
        }

        Err(FsError::NotSupported)
    }

    pub fn read(&self) -> RwSpinReadGuard<Ext2INode> {
        self.node.read()
    }

    pub fn d_inode_writer(&self) -> DINodeWriter {
        DINodeWriter {
            locked: self.write(),
            fs: self.fs.clone(),
            dirty: false,
        }
    }

    pub fn write(&self) -> RwSpinWriteGuard<Ext2INode> {
        self.node.write()
    }

    pub fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
    }

    pub fn unref_hardlink(&self) {
        let inode = self.node.read();
        let id = inode.id;
        let hl_count = inode.d_inode().hl_count();

        drop(inode);

        if hl_count > 0 {
            let mut writer = self.d_inode_writer();

            if writer.hl_count() > 0 {
                writer.dec_hl_count();
            }

            if hl_count == 1 {
                writer.set_deletion_time(crate::kernel::time::unix_timestamp() as u32);
            }
        }

        if hl_count == 1 {
            //It's 0 after decrement
            self.fs().drop_from_cache(id);
        }
    }
}

impl Drop for LockedExt2INode {
    fn drop(&mut self) {
        let inode = self.node.read();

        let hl_count = inode.d_inode.hl_count();

        drop(inode);

        if hl_count == 0 {
            self.fs().free_inode(self)
        }
    }
}

pub struct DINodeWriter<'a> {
    locked: RwSpinWriteGuard<'a, Ext2INode>,
    fs: Weak<Ext2Filesystem>,
    dirty: bool,
}

impl<'a> DINodeWriter<'a> {
    pub fn id(&self) -> usize {
        self.locked.id
    }
}

impl<'a> Deref for DINodeWriter<'a> {
    type Target = disk::inode::INode;

    fn deref(&self) -> &Self::Target {
        &self.locked.d_inode
    }
}

impl<'a> DerefMut for DINodeWriter<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.locked.d_inode
    }
}

impl<'a> Drop for DINodeWriter<'a> {
    fn drop(&mut self) {
        if let Some(fs) = self.fs.upgrade() {
            if self.dirty {
                fs.group_descs()
                    .write_d_inode(self.locked.id, &self.locked.d_inode);
            }
        }
    }
}

pub struct Ext2INode {
    id: usize,
    d_inode: disk::inode::INode,
}

impl From<disk::inode::FileType> for syscall_defs::FileType {
    fn from(v: disk::inode::FileType) -> Self {
        match v {
            disk::inode::FileType::File => FileType::File,
            disk::inode::FileType::Symlink => FileType::Symlink,
            disk::inode::FileType::Dir => FileType::Dir,
            disk::inode::FileType::BlockDev | disk::inode::FileType::CharDev => FileType::DevNode,
            _ => FileType::File,
        }
    }
}

impl Ext2INode {
    pub fn new(fs: Weak<Ext2Filesystem>, id: usize) -> Ext2INode {
        let mut inode = Ext2INode {
            id,
            d_inode: disk::inode::INode::default(),
        };

        fs.upgrade()
            .unwrap()
            .group_descs()
            .read_d_inode(id, &mut inode.d_inode);

        inode
    }

    pub fn d_inode(&self) -> &disk::inode::INode {
        &self.d_inode
    }

    pub fn d_inode_mut(&mut self) -> &mut disk::inode::INode {
        &mut self.d_inode
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn ftype(&self) -> syscall_defs::FileType {
        self.d_inode.ftype().into()
    }

    fn free_s_ptr(&mut self, ptr: usize, fs: &Ext2Filesystem) {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice_mut() {
            if *p != 0 {
                fs.group_descs().free_block_ptr(*p as usize);

                *p = 0;
            } else {
                break;
            }
        }
        fs.write_block(ptr, block.slice().to_bytes());

        fs.group_descs().free_block_ptr(ptr);
    }

    fn free_d_ptr(&mut self, ptr: usize, fs: &Ext2Filesystem) {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice_mut() {
            if *p != 0 {
                self.free_s_ptr(*p as usize, fs);

                *p = 0;
            } else {
                break;
            }
        }
        fs.write_block(ptr, block.slice().to_bytes());

        fs.group_descs().free_block_ptr(ptr);
    }
    fn free_t_ptr(&mut self, ptr: usize, fs: &Ext2Filesystem) {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice_mut() {
            if *p != 0 {
                self.free_d_ptr(*p as usize, fs);

                *p = 0;
            } else {
                break;
            }
        }
        fs.write_block(ptr, block.slice().to_bytes());

        fs.group_descs().free_block_ptr(ptr);
    }

    pub fn free_blocks(&mut self, fs: &Ext2Filesystem) {
        for i in 0usize..15 {
            let ptr = self.d_inode.block_ptrs()[i] as usize;

            if ptr == 0 {
                return;
            }

            if i < 12 {
                fs.group_descs().free_block_ptr(ptr);
            } else {
                match i {
                    12 => {
                        self.free_s_ptr(ptr, fs);
                    }
                    13 => {
                        self.free_d_ptr(ptr, fs);
                    }
                    14 => {
                        self.free_t_ptr(ptr, fs);
                    }
                    _ => unreachable!(),
                }
            }

            self.d_inode.block_ptrs_mut()[i] = 0;
        }

        fs.group_descs().write_d_inode(self.id, self.d_inode());
    }
}

impl INode for LockedExt2INode {
    fn metadata(&self) -> Result<Metadata> {
        let inode = self.read();
        Ok(Metadata {
            id: inode.id,
            typ: inode.d_inode.ftype().into(),
        })
    }

    fn lookup(&self, parent: Arc<DirEntry>, name: &str) -> Result<Arc<DirEntry>> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let mut iter = DirEntIter::new(self.self_ref.upgrade().unwrap());

        if let Some(e) = iter.find_map(|e| {
            if e.name() == name {
                Some(self.mk_dirent(parent.clone(), e))
            } else {
                None
            }
        }) {
            Ok(e)
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if DirEntIter::new(self.self_ref.upgrade().unwrap())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::Dir)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        if let Err(e) = iter.add_dir_entry(&new_inode, disk::inode::FileType::Dir, name) {
            self.fs().free_inode(&new_inode);

            Err(e)
        } else {
            Ok(new_inode)
        }
    }

    fn rmdir(&self, name: &str) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if [".", ".."].contains(&name) {
            return Err(FsError::NotSupported);
        }

        let this = self.self_ref.upgrade().unwrap();

        if this.id()? == 2 {
            return Err(FsError::NotSupported);
        }

        // Check if dir is not empty
        if DirEntIter::new(this.clone())
            .find(|e| ![".", ".."].contains(&e.name()))
            .is_some()
        {
            return Err(FsError::NotSupported);
        }

        if let Some(parent) =
            if let Some(e) = DirEntIter::new(this.clone()).find(|e| e.name() == "..") {
                Some(self.fs().get_inode(e.inode() as usize))
            } else {
                None
            }
        {
            let mut iter = DirEntIter::new(this);

            iter.remove_dir_entry(".")?;
            iter.remove_dir_entry("..")?;

            let mut iter = DirEntIter::new(parent.clone());

            iter.remove_dir_entry(name)?;
        }

        Ok(())
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if self.ftype()? != FileType::File && self.ftype()? != FileType::Symlink {
            return Err(FsError::NotFile);
        }

        let mut reader = INodeData::new(self.self_ref.upgrade().unwrap(), offset);

        Ok(reader.read(buf))
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        if self.ftype()? != FileType::File && self.ftype()? != FileType::Symlink {
            return Err(FsError::NotFile);
        }

        let mut writer = INodeData::new(self.self_ref.upgrade().unwrap(), offset);

        Ok(writer.write(buf)?)
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.fs()
    }

    fn create(
        &self,
        parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        name: &str,
    ) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if DirEntIter::new(self.self_ref.upgrade().unwrap())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::File)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        if let Err(e) = iter.add_dir_entry(&new_inode, disk::inode::FileType::File, name) {
            self.fs().free_inode(&new_inode);

            Err(e)
        } else {
            Ok(DirEntry::new(parent, new_inode, String::from(name)))
        }
    }

    fn symlink(&self, name: &str, target: &str) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if DirEntIter::new(self.self_ref.upgrade().unwrap())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::Symlink)?;

        if let Err(e) = new_inode.write_at(0, target.as_bytes()) {
            self.fs().free_inode(&new_inode);

            return Err(e);
        }

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        if let Err(e) = iter.add_dir_entry(&new_inode, disk::inode::FileType::Symlink, name) {
            self.fs().free_inode(&new_inode);

            Err(e)
        } else {
            Ok(())
        }
    }

    fn truncate(&self) -> Result<()> {
        if self.ftype()? != FileType::File {
            return Err(FsError::NotFile);
        }

        self.node.write().free_blocks(&self.fs());

        let mut node = self.d_inode_writer();

        node.set_size_lower(0);
        node.set_sector_count(0);

        Ok(())
    }

    fn dir_ent(&self, parent: Arc<DirEntry>, idx: usize) -> Result<Option<Arc<DirEntry>>> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let mut iter = DirEntIter::new(self.self_ref.upgrade().unwrap());

        if let Some(de) = iter.nth(idx) {
            Ok(Some(self.mk_dirent(parent, de)))
        } else {
            Ok(None)
        }
    }

    fn dir_iter(
        &self,
        parent: Arc<DirEntry>,
    ) -> Option<Arc<dyn crate::kernel::fs::vfs::DirEntIter>> {
        if self.ftype().ok()? != FileType::Dir {
            return None;
        }

        Some(Arc::new(SysDirEntIter::new(
            parent,
            self.self_ref.upgrade().unwrap(),
        )))
    }
}
