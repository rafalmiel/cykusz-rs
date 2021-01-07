use crate::kernel::fs::ext2::disk;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::fs::vfs::{DirEntry, Metadata};
use alloc::sync::{Arc, Weak};
use syscall_defs::FileType;

use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::arch::raw::mm::MappedAddr;
use crate::kernel::mm::Frame;
use crate::kernel::sched::current_task;
use alloc::string::String;

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
}

impl INode for Ext2INode {
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            id: self.id,
            typ: self.typ,
        })
    }

    fn dirent(&self, idx: usize) -> Result<Option<DirEntry>> {
        let _task = current_task();
        let fs = self.fs();

        let igroup = fs.group_descs().get_d_inode(self.id);
        let inodeg = igroup.read();

        let inode = inodeg.get(self.id);

        let vec = unsafe {
            allocate_order(0)
                .unwrap()
                .address_mapped()
                .as_bytes_mut(1024)
        };

        let _bytes = fs
            .dev()
            .read(inode.direct_ptr0() as usize * 2, vec)
            .unwrap();

        drop(inodeg);

        let mut i = idx;
        let mut offset = 0;

        while i > 0 && offset < 1024 {
            let de = unsafe { &*(vec[offset..].as_ptr() as *const super::disk::dirent::DirEntry) };

            offset += de.ent_size() as usize;
            i -= 1;
        }

        if offset < 1024 {
            let de = unsafe { &*(vec[offset..].as_ptr() as *const disk::dirent::DirEntry) };

            let typ = match de.ftype() {
                disk::dirent::FileType::RegularFile => FileType::File,
                disk::dirent::FileType::CharDev => FileType::DevNode,
                disk::dirent::FileType::Directory => FileType::Dir,
                _ => FileType::File,
            };

            let res = Ok(Some(DirEntry {
                name: String::from(de.name()),
                inode: Ext2INode::new(self.fs.clone(), de.inode() as usize, typ),
            }));

            deallocate_order(&Frame::new(MappedAddr(vec.as_ptr() as usize).to_phys()), 0);
            res
        } else {
            deallocate_order(&Frame::new(MappedAddr(vec.as_ptr() as usize).to_phys()), 0);

            Ok(None)
        }
    }
}
