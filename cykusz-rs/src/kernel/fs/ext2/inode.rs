use crate::kernel::fs::ext2::dirent::DirEntIter;
use crate::kernel::fs::ext2::disk;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{DirEntry, Metadata};
use crate::kernel::fs::vfs::{FsError, Result};
use alloc::sync::{Arc, Weak};
use syscall_defs::FileType;

use alloc::string::String;

use crate::kernel::fs::ext2::reader::INodeReader;

pub struct Ext2INode {
    id: usize,
    fs: Weak<Ext2Filesystem>,
    typ: FileType,
}

impl Ext2INode {
    pub fn new(fs: Weak<Ext2Filesystem>, id: usize, typ: FileType) -> Arc<Ext2INode> {
        let inode = Ext2INode { id, fs, typ };

        let i = Arc::new(inode);

        i
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
    }

    fn mk_dirent(&self, de: &disk::dirent::DirEntry) -> DirEntry {
        let typ = match de.ftype() {
            disk::dirent::DirEntTypeIndicator::RegularFile => FileType::File,
            disk::dirent::DirEntTypeIndicator::CharDev => FileType::DevNode,
            disk::dirent::DirEntTypeIndicator::Directory => FileType::Dir,
            disk::dirent::DirEntTypeIndicator::Symlink => FileType::Symlink,
            _ => FileType::File,
        };

        DirEntry {
            name: String::from(de.name()),
            inode: Ext2INode::new(self.fs.clone(), de.inode() as usize, typ),
        }
    }
}

impl INode for Ext2INode {
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            id: self.id,
            typ: self.typ,
        })
    }

    fn lookup(&self, name: &str) -> Result<DirEntry> {
        let fs = self.fs();

        let igroup = fs.group_descs().get_d_inode(self.id);
        let inodeg = igroup.read();

        let inode = inodeg.get(self.id);

        let mut iter = DirEntIter::new(self.fs.clone(), inode);

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
        if self.typ != FileType::File && self.typ != FileType::Symlink {
            return Err(FsError::NotSupported);
        }

        let fs = self.fs();

        let igroup = fs.group_descs().get_d_inode(self.id);
        let inodeg = igroup.read();

        let inode = inodeg.get(self.id);

        let mut reader = INodeReader::new(inode, self.fs.clone(), offset);

        Ok(reader.read(buf))
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.fs()
    }

    fn dirent(&self, idx: usize) -> Result<Option<DirEntry>> {
        let fs = self.fs();

        let igroup = fs.group_descs().get_d_inode(self.id);
        let inodeg = igroup.read();

        let inode = inodeg.get(self.id);

        let mut iter = DirEntIter::new(self.fs.clone(), inode);

        if let Some(de) = iter.nth(idx) {
            Ok(Some(self.mk_dirent(de)))
        } else {
            Ok(None)
        }
    }
}