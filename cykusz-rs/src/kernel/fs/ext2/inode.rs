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
                fs.free_inode(new);
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

    fn free_s_ptr(ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for &p in block.slice() {
            if p != 0 {
                fs.group_descs().free_block_ptr(p as usize);
            } else {
                break;
            }
        }
        fs.group_descs().free_block_ptr(ptr);
    }

    fn free_d_ptr(ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for &p in block.slice() {
            if p != 0 {
                Self::free_s_ptr(p as usize, fs);
            } else {
                break;
            }
        }
        fs.group_descs().free_block_ptr(ptr);
    }
    fn free_t_ptr(ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for &p in block.slice() {
            if p != 0 {
                Self::free_d_ptr(p as usize, fs);
            } else {
                break;
            }
        }
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
                    12 => Self::free_s_ptr(ptr, fs),
                    13 => Self::free_d_ptr(ptr, fs),
                    14 => Self::free_t_ptr(ptr, fs),
                    _ => unreachable!(),
                }
            }

            self.d_inode.block_ptrs_mut()[i] = 0;
        }
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
        if DirEntIter::new(self.self_ref.upgrade().unwrap())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::Dir)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        if let Err(e) = iter.add_dir_entry(&new_inode, disk::inode::FileType::Dir, name) {
            self.fs().free_inode(new_inode);

            Err(e)
        } else {
            Ok(new_inode)
        }
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        {
            let inode = self.node.read();

            if inode.ftype() != FileType::File && inode.ftype() != FileType::Symlink {
                return Err(FsError::NotSupported);
            }
        }

        let mut reader = INodeData::new(self.self_ref.upgrade().unwrap(), offset);

        Ok(reader.read(buf))
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        {
            let inode = self.node.read();

            if inode.ftype() != FileType::File {
                return Err(FsError::NotSupported);
            }
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
        if DirEntIter::new(self.self_ref.upgrade().unwrap())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::File)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        if let Err(e) = iter.add_dir_entry(&new_inode, disk::inode::FileType::File, name) {
            self.fs().free_inode(new_inode);

            Err(e)
        } else {
            Ok(DirEntry::new(parent, new_inode, String::from(name)))
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
        Some(Arc::new(SysDirEntIter::new(
            parent,
            self.self_ref.upgrade().unwrap(),
        )))
    }
}
