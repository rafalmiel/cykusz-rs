use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::ops::{Deref, DerefMut};

use syscall_defs::FileType;

use crate::kernel::fs::ext2::dirent::{DirEntIter, SysDirEntIter};
use crate::kernel::fs::ext2::disk;

use crate::kernel::fs::ext2::reader::INodeReader;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{DirEntry, Metadata};
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

    pub fn mk_dirent(&self, de: &disk::dirent::DirEntry) -> DirEntry {
        let inode = self.fs().get_inode(de.inode() as usize);

        DirEntry {
            name: String::from(de.name()),
            inode,
        }
    }

    pub fn mk_inode(&self, typ: FileType) -> Result<Arc<LockedExt2INode>> {
        match typ {
            FileType::Dir => self.mk_dir_inode(),
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn mk_dir_inode(&self) -> Result<Arc<LockedExt2INode>> {
        let fs = self.fs();

        let parent_id = self.id().expect("Failed to get id");

        if let Some(new) = fs.alloc_inode(parent_id) {
            let mut inner = new.d_inode_writer();

            *inner = disk::inode::INode::default();

            inner.set_ftype(disk::inode::FileType::Dir);
            inner.set_perm(0o755);
            inner.set_user_id(0);
            inner.set_group_id(0);

            let time = crate::kernel::time::unix_timestamp();
            inner.set_creation_time(time as u32);
            inner.set_last_modification(time as u32);
            inner.set_last_access(time as u32);

            drop(inner);

            let mut iter = DirEntIter::new_no_skip(new.clone());

            iter.add_dir_entry(&new, disk::inode::FileType::Dir, ".")?;
            iter.add_dir_entry(&self, disk::inode::FileType::Dir, "..")?;

            return Ok(new);
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
}

impl INode for LockedExt2INode {
    fn metadata(&self) -> Result<Metadata> {
        let inode = self.read();
        Ok(Metadata {
            id: inode.id,
            typ: inode.d_inode.ftype().into(),
        })
    }

    fn lookup(&self, name: &str) -> Result<DirEntry> {
        let mut iter = DirEntIter::new(self.self_ref.upgrade().unwrap());

        if let Some(e) = iter.find_map(|e| {
            if e.name() == name {
                Some(self.mk_dirent(e))
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

        let new_inode = self.mk_dir_inode()?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref.upgrade().unwrap());

        iter.add_dir_entry(&new_inode, disk::inode::FileType::Dir, name)?;

        self.fs()
            .group_descs()
            .inc_dir_count(new_inode.node.read().id);

        Ok(new_inode)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        {
            let inode = self.node.read();

            if inode.ftype() != FileType::File && inode.ftype() != FileType::Symlink {
                return Err(FsError::NotSupported);
            }
        }

        let mut reader = INodeReader::new(self.self_ref.upgrade().unwrap(), offset);

        Ok(reader.read(buf))
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.fs()
    }

    fn dir_ent(&self, idx: usize) -> Result<Option<DirEntry>> {
        let mut iter = DirEntIter::new(self.self_ref.upgrade().unwrap());

        if let Some(de) = iter.nth(idx) {
            Ok(Some(self.mk_dirent(de)))
        } else {
            Ok(None)
        }
    }

    fn dir_iter(&self) -> Option<Arc<dyn crate::kernel::fs::vfs::DirEntIter>> {
        Some(Arc::new(SysDirEntIter::new(
            self.self_ref.upgrade().unwrap(),
        )))
    }
}
