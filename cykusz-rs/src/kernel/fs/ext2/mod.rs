use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::size_of;

use crate::kernel::block::BlockDev;
use crate::kernel::fs::ext2::blockgroup::BlockGroupDescriptor;
use crate::kernel::fs::ext2::superblock::Superblock;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::sync::Spin;
use crate::kernel::utils::slice::ToBytesMut;

mod blockgroup;
mod dirent;
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

        let block_group = 0;
        let idx = 1;

        let inode_tbl =
            self.block_desc[block_group as usize].inode_table() as usize * self.sectors_per_block;

        let mut vec = Vec::<inode::INode>::new();
        vec.resize_with(4, || inode::INode::default());
        self.dev.read(inode_tbl, vec.as_mut_slice().to_bytes_mut());

        let inode = &vec[idx];

        let mut dirent = Vec::<u8>::new();
        dirent.resize(inode.sector_count() as usize * 512, 0);

        self.dev
            .read(inode.direct_ptr0() as usize * 2, dirent.as_mut_slice());

        let mut offset = 0;

        while offset < dirent.len() as isize {
            let ent = unsafe { &*(dirent.as_ptr().offset(offset) as *const dirent::DirEntry) };

            //println!("{:?}", ent);
            println!("dirent name {}", ent.name());

            offset += ent.ent_size() as isize;
        }

        //for b in self.block_desc.iter() {
        //    let bub = b.inode_table() as usize * self.sectors_per_block;

        //    let mut bitmap = Vec::<inode::INode>::new();
        //    bitmap.resize_with(self.superblock.inodes_in_group() as usize, || {
        //        inode::INode::default()
        //    });

        //    self.dev.read(bub, bitmap.as_mut_slice().to_bytes_mut());

        //    for b in bitmap.iter() {
        //        if b.direct_ptr0 != 0 {
        //            println!("{:?}", b);
        //        }
        //    }
        //}
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
