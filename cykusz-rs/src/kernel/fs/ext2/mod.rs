use alloc::sync::{Arc, Weak};

use crate::kernel::block::BlockDev;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use syscall_defs::FileType;

mod blockgroup;
mod dirent;
mod disk;
mod inode;
mod superblock;

pub struct Ext2Filesystem {
    self_ref: Weak<Ext2Filesystem>,
    dev: Arc<dyn BlockDev>,
    superblock: superblock::Superblock,
    blockgroupdesc: blockgroup::BlockGroupDescriptors,
}

impl Ext2Filesystem {
    fn wrap(self) -> Arc<Ext2Filesystem> {
        let fs = Arc::new(self);
        let weak = Arc::downgrade(&fs);
        let ptr = Arc::into_raw(fs) as *mut Self;

        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
        }
    }

    pub fn new(dev: Arc<dyn BlockDev>) -> Arc<dyn Filesystem> {
        let a = Ext2Filesystem {
            self_ref: Weak::new(),
            dev,
            superblock: superblock::Superblock::new(),
            blockgroupdesc: blockgroup::BlockGroupDescriptors::new(),
        }
        .wrap();

        a.init();

        a
    }

    fn dev(&self) -> &Arc<dyn BlockDev> {
        &self.dev
    }

    fn init(&self) {
        self.superblock.init(self.self_ref.clone());
        self.blockgroupdesc.init(self.self_ref.clone());
    }

    pub fn superblock(&self) -> &superblock::Superblock {
        &self.superblock
    }

    pub fn group_descs(&self) -> &blockgroup::BlockGroupDescriptors {
        &self.blockgroupdesc
    }
}

impl Filesystem for Ext2Filesystem {
    fn root_inode(&self) -> Arc<dyn INode> {
        inode::Ext2INode::new(self.self_ref.clone(), 2, FileType::Dir)
    }
}
