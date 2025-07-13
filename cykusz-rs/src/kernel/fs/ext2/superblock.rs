use alloc::sync::{Arc, Weak};

use spin::Once;

use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard};

use super::disk;

pub struct Superblock {
    d_superblock: RwMutex<disk::superblock::Superblock>,
    fs: Once<Weak<super::Ext2Filesystem>>,
}

impl Superblock {
    pub fn new() -> Superblock {
        Superblock {
            d_superblock: RwMutex::new(disk::superblock::Superblock::default()),
            fs: Once::new(),
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.get().unwrap().upgrade().unwrap()
    }

    pub fn init(&self, fs: Weak<Ext2Filesystem>) -> bool {
        self.fs.call_once(|| fs);

        let mut sb = self.write_inner();

        let fs = self.fs();

        let dev = fs.dev();

        if current_task_ref().locks() > 0 {
            logln!("sb init: locks > 0");
        }
        dev.read_cached(2 * 512, sb.as_bytes_mut())
            .expect("Failed to get ext2 superblock");

        sb.ext_sig() == 0xef53
    }

    pub fn group_count(&self) -> usize {
        self.read_inner().group_count()
    }

    pub fn inodes_per_group(&self) -> usize {
        self.read_inner().inodes_in_group() as usize
    }

    pub fn inode_size(&self) -> usize {
        self.read_inner().inode_size() as usize
    }

    pub fn inodes_per_block(&self) -> usize {
        self.read_inner().inodes_per_block()
    }

    pub fn sectors_per_block(&self) -> usize {
        self.read_inner().sectors_per_block()
    }

    pub fn block_size(&self) -> usize {
        self.read_inner().block_size()
    }

    pub fn blocks_in_group(&self) -> usize {
        self.read_inner().blocks_in_group() as usize
    }

    pub fn read_inner(&self) -> RwMutexReadGuard<'_, disk::superblock::Superblock> {
        self.d_superblock.read()
    }
    pub fn write_inner(&self) -> RwMutexWriteGuard<'_, disk::superblock::Superblock> {
        self.d_superblock.write()
    }

    pub fn sync(&self, fs: &Ext2Filesystem) {
        let sb = self.read_inner();

        fs.dev.update_cached(2 * 512, sb.as_bytes());
    }

    pub fn debug(&self) {
        logln_disabled!(
            "SIZE: {}",
            core::mem::size_of::<disk::superblock::Superblock>()
        );
        logln_disabled!("{:?}", *self.read_inner());
    }

    pub fn block_groups_sector(&self) -> usize {
        match self.block_size() {
            1024 => 4,
            a if a > 1024 => a / 512,
            _ => unreachable!(),
        }
    }

    pub fn first_block(&self) -> usize {
        match self.block_size() {
            1024 => 1,
            a if a > 1024 => 0,
            _ => panic!("invalid block size"),
        }
    }
}
