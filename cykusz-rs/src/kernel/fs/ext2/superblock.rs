use alloc::sync::{Arc, Weak};

use spin::Once;

use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::{RwSpin, RwSpinReadGuard};

use super::disk;

pub struct Superblock {
    d_superblock: RwSpin<disk::superblock::Superblock>,
    fs: Once<Weak<super::Ext2Filesystem>>,
}

impl Superblock {
    pub fn new() -> Superblock {
        Superblock {
            d_superblock: RwSpin::new(disk::superblock::Superblock::default()),
            fs: Once::new(),
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.get().unwrap().upgrade().unwrap()
    }

    pub fn init(&self, fs: Weak<Ext2Filesystem>) -> bool {
        self.fs.call_once(|| fs);

        let mut sb = self.d_superblock.write();

        let fs = self.fs();

        let dev = fs.dev();

        dev.read(2, sb.as_bytes_mut())
            .expect("Failed to get ext2 superblock");

        sb.ext_sig() == 0xef53
    }

    pub fn group_count(&self) -> usize {
        self.d_superblock.read().group_count()
    }

    pub fn inodes_per_group(&self) -> usize {
        self.d_superblock.read().inodes_in_group() as usize
    }

    pub fn inodes_per_block(&self) -> usize {
        self.d_superblock.read().inodes_per_block()
    }

    pub fn sectors_per_block(&self) -> usize {
        self.d_superblock.read().sectors_per_block()
    }

    pub fn block_size(&self) -> usize {
        self.d_superblock.read().block_size()
    }

    pub fn read_inner(&self) -> RwSpinReadGuard<disk::superblock::Superblock> {
        self.d_superblock.read()
    }

    pub fn sync(&self, fs: &Ext2Filesystem) {
        let sb = self.read_inner();

        fs.dev.write(2, sb.as_bytes());
    }
}
