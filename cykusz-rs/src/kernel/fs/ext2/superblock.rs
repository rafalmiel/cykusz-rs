use alloc::sync::{Arc, Weak};

use spin::Once;

use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::{RwSpin, RwSpinReadGuard, RwSpinWriteGuard};

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

        dev.read_cached(2 * 512, sb.as_bytes_mut())
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

    pub fn blocks_in_group(&self) -> usize {
        self.d_superblock.read().blocks_in_group() as usize
    }

    pub fn read_inner(&self) -> RwSpinReadGuard<disk::superblock::Superblock> {
        self.d_superblock.read()
    }
    pub fn write_inner(&self) -> RwSpinWriteGuard<disk::superblock::Superblock> {
        self.d_superblock.write()
    }

    pub fn sync(&self, fs: &Ext2Filesystem) {
        let sb = self.read_inner();

        fs.dev.write_cached(2 * 512, sb.as_bytes());
    }

    pub fn debug(&self) {
        println!(
            "SIZE: {}",
            core::mem::size_of::<disk::superblock::Superblock>()
        );
        println!("{:?}", *self.d_superblock.read());
    }

    pub fn block_groups_sector(&self) -> usize {
        match self.block_size() {
            1024 => 4,
            a if a > 1024 => a / 512,
            _ => unreachable!(),
        }
    }

    pub fn first_block(&self) -> usize {
        1
    }
}
