use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use spin::Once;

use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::{Mutex, RwSpin, RwSpinReadGuard, RwSpinWriteGuard};
use crate::kernel::utils::slice::{ToBytes, ToBytesMut};

use super::disk;
use crate::kernel::fs::ext2::disk::blockgroup::BlockGroupDescriptor;
use bit_field::BitField;
use core::ops::Index;
use core::ops::IndexMut;

pub struct INodeVec {
    vec: Vec<disk::inode::INode>,
    src_block: usize,
    dirty: bool,
}

pub struct INodeGroup {
    fs: Weak<Ext2Filesystem>,
    inodes: RwSpin<INodeVec>,
}

impl INodeVec {
    pub fn get(&self, id: usize) -> &disk::inode::INode {
        &self.vec[(id - 1) % self.vec.len()]
    }
    pub fn get_mut(&mut self, id: usize) -> &mut disk::inode::INode {
        let len = self.vec.len();
        self.dirty = true;
        &mut self.vec[(id - 1) % len]
    }
}

impl INodeGroup {
    pub fn read(&self) -> RwSpinReadGuard<INodeVec> {
        self.inodes.read()
    }
    pub fn write(&self) -> RwSpinWriteGuard<INodeVec> {
        self.inodes.write()
    }
}

impl Drop for INodeGroup {
    fn drop(&mut self) {
        let l = self.inodes.read();

        if l.dirty {
            if let Some(fs) = self.fs.upgrade() {
                fs.write_block(l.src_block, l.vec.as_slice().to_bytes());
            }
        }
    }
}

struct GroupDescriptors {
    vec: Vec<disk::blockgroup::BlockGroupDescriptor>,
}

impl GroupDescriptors {
    fn new() -> GroupDescriptors {
        GroupDescriptors { vec: Vec::new() }
    }

    fn init(&mut self, fs: &Arc<Ext2Filesystem>) {
        let sb = fs.superblock();

        self.vec
            .resize(sb.group_count(), BlockGroupDescriptor::default());

        fs.dev()
            .read(
                sb.block_groups_sector(),
                self.vec.as_mut_slice().to_bytes_mut(),
            )
            .expect("Failed to load GroupDescriptors");
    }

    fn sync(&self, fs: &Ext2Filesystem) {
        let sb = fs.superblock();

        fs.dev()
            .write(sb.block_groups_sector(), self.vec.as_slice().to_bytes())
            .expect("Failed to sync GroupDescriptors");
    }

    fn find_free_blocks_group(&self, hint: usize) -> Option<usize> {
        if self.vec[hint].unallocated_blocks() > 0 {
            Some(hint)
        } else {
            let res = self
                .vec
                .iter()
                .enumerate()
                .max_by_key(|e| e.1.unallocated_blocks())
                .unwrap();

            if res.1.unallocated_blocks() > 0 {
                Some(res.0)
            } else {
                None
            }
        }
    }

    fn find_free_inodes_group(&self, hint: Option<usize>) -> Option<usize> {
        if let Some(h) = hint {
            if self.vec[h].unallocated_inodes() > 0 {
                return Some(h);
            }
        }

        let res = self
            .vec
            .iter()
            .enumerate()
            .max_by_key(|e| e.1.unallocated_inodes())
            .unwrap();

        if res.1.unallocated_inodes() > 0 {
            Some(res.0)
        } else {
            None
        }
    }
}

impl Index<usize> for GroupDescriptors {
    type Output = disk::blockgroup::BlockGroupDescriptor;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

impl IndexMut<usize> for GroupDescriptors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vec[index]
    }
}

struct BlocksBitmap {
    map: Vec<u8>,
}

impl BlocksBitmap {
    fn new(fs: &Ext2Filesystem) -> BlocksBitmap {
        let mut vec = Vec::<u8>::new();
        vec.resize(fs.superblock().block_size(), 0);

        BlocksBitmap { map: vec }
    }

    fn new_from_block(fs: &Ext2Filesystem, block: usize) -> BlocksBitmap {
        let mut bm = BlocksBitmap::new(fs);

        fs.read_block(block, bm.map.as_mut_slice())
            .expect("Failed to load BlocksBitmap");

        bm
    }

    fn alloc_bit(&mut self) -> Option<usize> {
        for (i, el) in self.map.iter_mut().enumerate() {
            if *el != 0xff {
                for bit in 0..8 {
                    if el.get_bit(bit) == false {
                        el.set_bit(bit, true);

                        return Some(i * 8 + bit);
                    }
                }
            }
        }

        None
    }

    fn sync(&self, fs: &Ext2Filesystem, block: usize) {
        fs.write_block(block, self.map.as_slice())
            .expect("Failed to sync BlocksBitmap");
    }
}

pub struct BlockGroupDescriptors {
    d_desc: RwSpin<GroupDescriptors>,
    d_inodes: Mutex<lru::LruCache<usize, Arc<INodeGroup>>>,
    fs: Once<Weak<super::Ext2Filesystem>>,
}

impl BlockGroupDescriptors {
    pub fn new() -> BlockGroupDescriptors {
        BlockGroupDescriptors {
            d_desc: RwSpin::new(GroupDescriptors::new()),
            d_inodes: Mutex::new(lru::LruCache::new(256)),
            fs: Once::new(),
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.get().unwrap().upgrade().unwrap()
    }

    pub fn init(&self, fs: Weak<Ext2Filesystem>) {
        self.fs.call_once(|| fs);

        let fs = self.fs();

        let mut desc = self.d_desc.write();

        desc.init(&fs);
    }

    fn get_inode_block(&self, id: usize) -> usize {
        let fs = self.fs();

        let (ipg, ipb) = {
            let sb = fs.superblock().read_inner();
            (sb.inodes_in_group() as usize, sb.inodes_per_block())
        };

        let bg_idx = (id - 1) / ipg;
        let idx = (id - 1) % ipg;
        let block_off = idx / ipb;

        let desc = self.d_desc.read();

        desc[bg_idx].inode_table() as usize + block_off
    }

    pub fn get_d_inode(&self, id: usize) -> Arc<INodeGroup> {
        let block = self.get_inode_block(id);

        let mut inodes = self.d_inodes.lock();

        if let Some(e) = inodes.get(&block) {
            e.clone()
        } else {
            let fs = self.fs();
            let sb = fs.superblock().read_inner();

            //load
            let mut vec = Vec::<disk::inode::INode>::new();
            vec.resize(sb.inodes_per_block(), disk::inode::INode::default());

            fs.read_block(block, vec.as_mut_slice().to_bytes_mut());

            let res = Arc::new(INodeGroup {
                fs: Arc::downgrade(&fs),
                inodes: RwSpin::new(INodeVec {
                    vec,
                    src_block: block,
                    dirty: false,
                }),
            });

            inodes.put(block, res.clone());

            res
        }
    }

    pub fn write_d_inode(&self, id: usize, d_inode: &disk::inode::INode) {
        let group = self.get_d_inode(id);
        let mut vec = group.write();

        *vec.get_mut(id) = *d_inode;
    }

    pub fn read_d_inode(&self, id: usize, d_inode: &mut disk::inode::INode) {
        let group = self.get_d_inode(id);
        let vec = group.read();

        *d_inode = *vec.get(id);
    }

    pub fn debug(&self) {
        let l = self.d_desc.read();

        for d in l.vec.iter() {
            println!("{:?}", d);
        }
    }

    pub fn alloc_block_ptr(&self, hint_id: usize) -> Option<usize> {
        let fs = self.fs();
        let sb = fs.superblock();

        let blocks_in_group = sb.blocks_in_group();
        let inodes_in_group = sb.inodes_per_group();

        let bg_idx = (hint_id - 1) / inodes_in_group;

        let mut bg = self.d_desc.write();

        if let Some(found_bg) = bg.find_free_blocks_group(bg_idx) {
            let bgroup = &mut bg[found_bg];

            let block = bgroup.block_usage_bitmap() as usize;

            let mut bmap = BlocksBitmap::new_from_block(&fs, block);

            if let Some(block_nr) = bmap.alloc_bit() {
                bmap.sync(&fs, block);

                let id = found_bg * blocks_in_group + block_nr + sb.first_block();

                bgroup.dec_unallocated_blocks();

                let mut sb = sb.write_inner();
                sb.dec_free_blocks();

                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn alloc_inode_id(&self, hint_id: usize) -> Option<usize> {
        let fs = self.fs();
        let sb = fs.superblock();

        let inodes_per_group = sb.inodes_per_group();

        let bg_idx = if hint_id != 2 {
            Some((hint_id - 1) / inodes_per_group)
        } else {
            None
        };

        let mut bg = self.d_desc.write();

        if let Some(found_bg) = bg.find_free_inodes_group(bg_idx) {
            let bgroup = &mut bg[found_bg];

            let block = bgroup.inode_usage_bitmap() as usize;

            let mut bmap = BlocksBitmap::new_from_block(&fs, block);

            if let Some(inode) = bmap.alloc_bit() {
                bmap.sync(&fs, block);

                let id = found_bg * inodes_per_group + inode + 1;

                bgroup.dec_unallocated_inodes();

                let mut sb = sb.write_inner();
                sb.dec_free_inodes();

                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn inc_dir_count(&self, inode: usize) {
        let fs = self.fs();
        let ipg = fs.superblock().inodes_per_group();

        let idx = (inode - 1) / ipg;

        self.d_desc.write().vec[idx].inc_dir_count();
    }

    pub fn dec_dir_count(&self, inode: usize) {
        let fs = self.fs();
        let ipg = fs.superblock().inodes_per_group();

        let idx = (inode - 1) / ipg;

        self.d_desc.write().vec[idx].dec_dir_count();
    }

    pub fn sync(&self, fs: &Ext2Filesystem) {
        let desc = self.d_desc.read();

        desc.sync(fs);

        let iter = self.d_inodes.lock();

        for (block, e) in iter.iter() {
            let mut el = e.write();

            if el.dirty {
                fs.write_block(*block, el.vec.as_slice().to_bytes())
                    .expect("Failed to sync inode group");

                el.dirty = false;
            }
        }
    }
}
