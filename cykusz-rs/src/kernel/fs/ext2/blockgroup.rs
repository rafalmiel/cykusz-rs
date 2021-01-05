#![allow(dead_code)]

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct BlockGroupDescriptor {
    block_usage_bitmap: u32,
    inode_usage_bitmap: u32,
    inode_table: u32,
    unallocated_blocks: u16,
    unallocated_inodes: u16,
    dir_count: u16,
    _unused: [u8; 14],
}

impl BlockGroupDescriptor {
    pub fn block_usage_bitmap(&self) -> u32 {
        self.block_usage_bitmap
    }
    pub fn inode_usage_bitmap(&self) -> u32 {
        self.inode_usage_bitmap
    }
    pub fn inode_table(&self) -> u32 {
        self.inode_table
    }
    pub fn unallocated_blocks(&self) -> u16 {
        self.unallocated_blocks
    }
    pub fn unallocated_inodes(&self) -> u16 {
        self.unallocated_inodes
    }
    pub fn dir_count(&self) -> u16 {
        self.dir_count
    }
}
