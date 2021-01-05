use crate::kernel::block::BlockDev;
use crate::kernel::fs::ext2::blockgroup::BlockGroupDescriptor;
use crate::kernel::fs::ext2::superblock::Superblock;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::sync::Spin;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::size_of;

mod blockgroup;
mod inode;
mod superblock;

struct Ext2FilesystemData {
    dev: Arc<dyn BlockDev>,
    superblock: Box<Superblock>,
    block_desc: Vec<BlockGroupDescriptor>,
    sectors_per_block: usize,
}

pub struct Ext2Filesystem {
    data: Spin<Ext2FilesystemData>,
}

struct Ext2INode {}

impl INode for Ext2INode {}

impl Ext2FilesystemData {
    fn init(&mut self) {
        if let None = self.dev.read(2, self.superblock.as_bytes_mut()) {
            panic!("Failed to initialize ext2 filesystem");
        }
        println!("group count: {}", self.superblock.group_count());

        self.sectors_per_block = self.superblock.block_size() / 512;

        self.block_desc
            .resize_with(self.superblock.group_count(), || {
                BlockGroupDescriptor::default()
            });

        if let None = self.dev.read(4, unsafe {
            core::slice::from_raw_parts_mut(
                self.block_desc.as_mut_ptr() as *mut u8,
                self.block_desc.len() * size_of::<BlockGroupDescriptor>(),
            )
        }) {
            panic!("Failed to read BlockGroupDesc");
        }

        for b in self.block_desc.iter() {
            println!("{:?}", b);
            let bub = b.inode_table() as usize * self.sectors_per_block;

            let mut bitmap = Vec::<inode::INode>::new();
            bitmap.resize_with(self.superblock.inodes_in_group() as usize, || inode::INode::default());

            println!("Reading sector {} with {} bytes", bub, bitmap.len() * core::mem::size_of::<inode::INode>());
            self.dev.read(bub, unsafe {
                core::slice::from_raw_parts_mut(
                    bitmap.as_mut_ptr() as *mut u8,
                    bitmap.len() * core::mem::size_of::<inode::INode>(),
                )
            });

            for b in bitmap.iter() {
                if b.direct_ptr0 != 0 {
                    println!("{:?}", b);
                }
            }
        }
    }
}

impl Ext2Filesystem {
    pub fn new(dev: Arc<dyn BlockDev>) -> Arc<dyn Filesystem> {
        let a = Arc::new(Ext2Filesystem {
            data: Spin::new(Ext2FilesystemData {
                dev,
                superblock: Box::new(Superblock::default()),
                block_desc: Vec::new(),
                sectors_per_block: 0,
            }),
        });

        a.init();

        a
    }

    fn init(&self) {
        self.data.lock().init();
    }
}

impl Filesystem for Ext2Filesystem {
    fn root_inode(&self) -> Arc<dyn INode> {
        Arc::new(Ext2INode {})
    }
}
