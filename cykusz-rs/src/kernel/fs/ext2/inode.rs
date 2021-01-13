use alloc::string::String;
use alloc::sync::{Arc, Weak};

use syscall_defs::FileType;

use crate::kernel::fs::ext2::dirent::{DirEntIter, SysDirEntIter};
use crate::kernel::fs::ext2::disk;
use crate::kernel::fs::ext2::reader::INodeReader;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{DirEntry, Metadata};
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::{RwSpin, RwSpinReadGuard};

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
        DirEntry {
            name: String::from(de.name()),
            inode: self.fs().get_inode(de.inode() as usize),
        }
    }

    pub fn read(&self) -> RwSpinReadGuard<Ext2INode> {
        self.node.read()
    }

    pub fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
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
