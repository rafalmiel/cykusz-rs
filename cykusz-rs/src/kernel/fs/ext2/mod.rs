use alloc::string::String;
use alloc::sync::{Arc, Weak};

use spin::Once;

use crate::kernel::fs::dirent::{DirEntry, DirEntryItem};
use crate::kernel::fs::ext2::buf_block::{BufBlock, SliceBlock};
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemStruct};
use crate::kernel::fs::pcache::{CachedBlockDev, PageItemStruct};
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::{Mutex, MutexGuard};
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
    dev: Arc<dyn CachedBlockDev>,
    sectors_per_block: Once<usize>,
    superblock: superblock::Superblock,
    blockgroupdesc: blockgroup::BlockGroupDescriptors,
    dir_lock: Mutex<()>,
}

impl Ext2Filesystem {
    pub fn new(dev: Arc<dyn CachedBlockDev>) -> Option<Arc<dyn Filesystem>> {
        let a = Arc::new_cyclic(|me| Ext2Filesystem {
            self_ref: me.clone(),
            dev,
            sectors_per_block: Once::new(),
            superblock: superblock::Superblock::new(),
            blockgroupdesc: blockgroup::BlockGroupDescriptors::new(),
            dir_lock: Mutex::new(()),
        });

        if !a.init() {
            None
        } else {
            Some(a)
        }
    }

    fn dev(&self) -> &Arc<dyn CachedBlockDev> {
        &self.dev
    }

    fn init(&self) -> bool {
        if !self.superblock.init(self.self_ref.clone()) {
            return false;
        }
        self.sectors_per_block
            .call_once(|| self.superblock.sectors_per_block());
        self.blockgroupdesc.init(self.self_ref.clone());

        self.debug();

        true
    }

    fn sectors_per_block(&self) -> usize {
        *self.sectors_per_block.get().unwrap()
    }

    pub fn read_block(&self, block: usize, dest: &mut [u8]) -> Option<usize> {
        if current_task_ref().locks() > 0 {
            logln!("read_block: locks > 0");
        }
        self.dev
            .read_cached(block * self.sectors_per_block() * 512, dest)
    }

    pub fn sync_block(&self, block: usize) -> bool {
        let res = self.dev.sync_offset(block * self.sectors_per_block() * 512);

        res
    }

    pub fn write_block(&self, block: usize, buf: &[u8]) -> Option<usize> {
        self.dev
            .write_cached(block * self.sectors_per_block() * 512, buf)
    }

    pub fn write_block_sync(&self, block: usize, buf: &[u8], sync: bool) -> Option<usize> {
        self.dev.update_cached_synced(block * self.sectors_per_block() * 512, buf, sync)
    }

    pub fn dir_lock(&self) -> MutexGuard<()> {
        self.dir_lock.lock()
    }

    pub fn superblock(&self) -> &superblock::Superblock {
        &self.superblock
    }

    pub fn group_descs(&self) -> &blockgroup::BlockGroupDescriptors {
        &self.blockgroupdesc
    }

    pub fn get_inode(&self, id: usize) -> INodeItem {
        let el = inode::LockedExt2INode::new(self.self_ref.clone(), id);

        el
    }

    pub fn drop_from_cache(&self, id: usize) {
        let cache = crate::kernel::fs::icache::cache();

        let fs: Weak<dyn Filesystem> = self.self_ref.clone();

        cache.remove(&INodeItemStruct::make_key(&fs, id));
    }

    pub fn alloc_inode(&self, hint: usize) -> Option<INodeItem> {
        if let Some(id) = self.group_descs().alloc_inode_id(hint) {
            let inode = self.get_inode(id);

            Some(inode)
        } else {
            None
        }
    }

    pub fn free_inode(&self, inode: &LockedExt2INode) {
        logln_disabled!("Free inode: {}", inode.read_debug(33).id());

        inode.write_debug(17).free_blocks(self);

        let id = inode.read_debug(34).id();

        self.group_descs().free_inode_id(id);
    }

    pub fn alloc_block(&self, hint: usize) -> Option<BufBlock> {
        if let Some(ptr) = self.group_descs().alloc_block_ptr(hint) {
            let mut buf = self.make_buf();
            buf.set_block(ptr);

            logln!("allocated new block {}", ptr);

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

        let imp = i.as_impl::<LockedExt2INode>();

        let lock = imp.read_debug(35);

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
        //self.umount();
        //println!("ext2 fs drop")
    }
}

impl Filesystem for Ext2Filesystem {
    fn root_dentry(&self) -> DirEntryItem {
        let e = DirEntry::new_root(self.get_inode(2), String::from("/"));
        e.init_fs(self.self_ref.clone());
        e
    }

    fn sync(&self) {
        println!("[ EXT2 ] Syncing...");
        self.blockgroupdesc.sync(self);
        self.superblock.sync(self);

        self.dev.sync_all();
    }

    fn umount(&self) {
        println!("[ EXT2 ] Unmounting");
        self.blockgroupdesc.umount();

        self.sync();
    }

    fn name(&self) -> &'static str {
        "ext2"
    }
}

impl INodeItemStruct {
    pub(in crate::kernel::fs::ext2) fn as_ext2_inode(&self) -> &LockedExt2INode {
        self.as_impl::<LockedExt2INode>()
    }

    pub(in crate::kernel::fs::ext2) fn as_ext2_inode_arc(&self) -> Arc<LockedExt2INode> {
        self.as_arc::<LockedExt2INode>()
    }
}
