use alloc::string::String;
use alloc::sync::{Arc, Weak};

use spin::Once;

use crate::kernel::block::BlockDev;
use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::ext2::buf_block::{BufBlock, SliceBlock};
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::sync::Spin;
use crate::kernel::utils::slice::ToBytesMut;

mod blockgroup;
mod buf_block;
mod dirent;
mod disk;
mod idata;
mod inode;
mod superblock;

pub struct Ext2Filesystem {
    self_ref: Weak<Ext2Filesystem>,
    dev: Arc<dyn BlockDev>,
    sectors_per_block: Once<usize>,
    superblock: superblock::Superblock,
    blockgroupdesc: blockgroup::BlockGroupDescriptors,
    inode_cache: Spin<lru::LruCache<usize, Arc<inode::LockedExt2INode>>>,
}

impl Ext2Filesystem {
    pub fn new(dev: Arc<dyn BlockDev>) -> Option<Arc<dyn Filesystem>> {
        let a = Arc::new_cyclic(|me| Ext2Filesystem {
            self_ref: me.clone(),
            dev,
            sectors_per_block: Once::new(),
            superblock: superblock::Superblock::new(),
            blockgroupdesc: blockgroup::BlockGroupDescriptors::new(),
            inode_cache: Spin::new(lru::LruCache::new(256)),
        });

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

        //self.debug();

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

    pub fn get_inode(&self, id: usize) -> Arc<LockedExt2INode> {
        let mut cache = self.inode_cache.lock();

        if let Some(el) = cache.get(&id) {
            el.clone()
        } else {
            let el = inode::LockedExt2INode::new(self.self_ref.clone(), id);

            cache.put(id, el.clone());

            el
        }
    }

    pub fn drop_from_cache(&self, id: usize) {
        //println!("drop from ext2 cache: {}", id);

        let mut cache = self.inode_cache.lock();

        cache.pop(&id);
    }

    pub fn alloc_inode(&self, hint: usize) -> Option<Arc<LockedExt2INode>> {
        if let Some(id) = self.group_descs().alloc_inode_id(hint) {
            let inode = self.get_inode(id);

            Some(inode)
        } else {
            None
        }
    }

    pub fn free_inode(&self, inode: &LockedExt2INode) {
        inode.write().free_blocks(self);

        let id = inode.read().id();

        if inode.read().ftype() == syscall_defs::FileType::Dir {
            self.group_descs().dec_dir_count(id)
        }

        self.group_descs().free_inode_id(id);
    }

    pub fn alloc_block(&self, hint: usize) -> Option<BufBlock> {
        if let Some(ptr) = self.group_descs().alloc_block_ptr(hint) {
            let mut buf = self.make_buf();
            buf.set_block(ptr);

            Some(buf)
        } else {
            None
        }
    }

    pub fn make_buf(&self) -> BufBlock {
        BufBlock::new(self.superblock().block_size())
    }

    pub fn make_buf_from(&self, block: usize) -> BufBlock {
        let mut buf = self.make_buf();

        self.read_block(block, buf.slice_mut());
        buf.set_block(block);

        buf
    }

    pub fn make_slice_buf<T: Sized + Default + Copy>(&self) -> SliceBlock<T> {
        SliceBlock::<T>::new(self.superblock().block_size() / core::mem::size_of::<T>())
    }

    pub fn make_slice_buf_from<T: Sized + Default + Copy>(&self, block: usize) -> SliceBlock<T> {
        let mut slice =
            SliceBlock::<T>::new(self.superblock().block_size() / core::mem::size_of::<T>());

        self.read_block(block, slice.slice_mut().to_bytes_mut())
            .expect("Failed to read block");
        slice.set_block(block);

        slice
    }

    pub fn make_buf_size(&self, size: usize) -> BufBlock {
        BufBlock::new(size)
    }

    #[allow(dead_code)]
    fn debug(&self) {
        self.superblock.debug();
        self.blockgroupdesc.debug();
    }

    #[allow(dead_code)]
    fn debug_resize_inode(&self) {
        use alloc::vec::Vec;

        let i = self.get_inode(7);

        let lock = i.read();

        let d_inode = lock.d_inode();

        println!("{:?}", d_inode);

        let ptr = d_inode.d_indir_ptr();

        let mut vec = Vec::<u32>::new();
        vec.resize(1024 / 4, 0);

        self.read_block(ptr as usize, vec.as_mut_slice().to_bytes_mut());

        println!("{:?}", vec);

        let mut vec2 = Vec::<u32>::new();
        vec2.resize(1024 / 4, 0);

        for (i, p) in vec.iter().enumerate() {
            self.read_block(*p as usize, vec2.as_mut_slice().to_bytes_mut());

            println!("{}: {:?}", i, vec2);
        }
    }
}

impl Drop for Ext2Filesystem {
    fn drop(&mut self) {
        self.sync();
    }
}

impl Filesystem for Ext2Filesystem {
    fn root_dentry(&self) -> Arc<super::dirent::DirEntry> {
        let e = DirEntry::new_root(self.get_inode(2), String::from("/"));
        e.init_fs(self.self_ref.clone());
        e
    }

    fn sync(&self) {
        println!("[ EXT2 ] Syncing...");
        self.blockgroupdesc.sync(self);
        self.superblock.sync(self);
    }

    fn name(&self) -> &'static str {
        "ext2"
    }
}
