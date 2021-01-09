use alloc::sync::{Arc, Weak};

use crate::kernel::block::BlockDev;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use spin::Once;
use syscall_defs::FileType;

mod blockgroup;
mod dirent;
mod disk;
mod inode;
mod reader;
mod superblock;

pub struct Ext2Filesystem {
    self_ref: Weak<Ext2Filesystem>,
    dev: Arc<dyn BlockDev>,
    sectors_per_block: Once<usize>,
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

    pub fn new(dev: Arc<dyn BlockDev>) -> Option<Arc<dyn Filesystem>> {
        let a = Ext2Filesystem {
            self_ref: Weak::new(),
            dev,
            sectors_per_block: Once::new(),
            superblock: superblock::Superblock::new(),
            blockgroupdesc: blockgroup::BlockGroupDescriptors::new(),
        }
        .wrap();

        if !a.init() {
            None
        } else {
            Some(a)
        }
    }

    fn dev(&self) -> &Arc<dyn BlockDev> {
        &self.dev
    }

    fn init(&self) -> bool {
        if !self.superblock.init(self.self_ref.clone()) {
            return false;
        }
        self.sectors_per_block
            .call_once(|| self.superblock.sectors_per_block());
        self.blockgroupdesc.init(self.self_ref.clone());

        true
    }

    fn sectors_per_block(&self) -> usize {
        *self.sectors_per_block.get().unwrap()
    }

    pub fn read_block(&self, block: usize, dest: &mut [u8]) -> Option<usize> {
        self.dev.read(block * self.sectors_per_block(), dest)
    }

    pub fn write_block(&self, block: usize, buf: &[u8]) -> Option<usize> {
        self.dev.write(block * self.sectors_per_block(), buf)
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
