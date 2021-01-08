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
use alloc::vec::Vec;

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

    #[allow(dead_code)]
    fn test(&self) {
        let fs = self.fs();

        let group = fs.group_descs().get_d_inode(self.id);

        let inodes = group.read();

        println!("{:?}", inodes.get(self.id));
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
    }

    fn mk_dirent(&self, de: &disk::dirent::DirEntry) -> DirEntry {
        let typ = match de.ftype() {
            disk::dirent::FileType::RegularFile => FileType::File,
            disk::dirent::FileType::CharDev => FileType::DevNode,
            disk::dirent::FileType::Directory => FileType::Dir,
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
        if self.typ != FileType::File {
            return Err(FsError::NotSupported);
        }

        let fs = self.fs();

        let igroup = fs.group_descs().get_d_inode(self.id);
        let inodeg = igroup.read();

        let inode = inodeg.get(self.id);

        let size = core::cmp::min(1024, inode.size_lower() as usize);

        if offset >= size {
            return Ok(0);
        }

        let to_copy = core::cmp::min(buf.len(), size - offset);

        let ptr = inode.direct_ptr0();

        if ptr != 0 {
            let mut data = Vec::<u8>::new();
            data.resize(1024, 0);

            fs.dev.read(ptr as usize * 2, data.as_mut_slice());

            buf[..to_copy].copy_from_slice(&data.as_slice()[offset..offset + to_copy]);

            Ok(to_copy)
        } else {
            Err(FsError::NotSupported)
        }
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
