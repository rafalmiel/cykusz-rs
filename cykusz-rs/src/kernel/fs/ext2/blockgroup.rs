use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use spin::Once;

use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::{Mutex, RwSpin, RwSpinReadGuard, RwSpinWriteGuard};
use crate::kernel::utils::slice::{ToBytes, ToBytesMut};

use super::disk;

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
                println!("syncing block {}", l.src_block);

                fs.write_block(l.src_block, l.vec.as_slice().to_bytes());
            }
        }
    }
}

pub struct BlockGroupDescriptors {
    d_desc: RwSpin<Vec<disk::blockgroup::BlockGroupDescriptor>>,
    d_inodes: Mutex<lru::LruCache<usize, Arc<INodeGroup>>>,
    fs: Once<Weak<super::Ext2Filesystem>>,
}

impl BlockGroupDescriptors {
    pub fn new() -> BlockGroupDescriptors {
        BlockGroupDescriptors {
            d_desc: RwSpin::new(Vec::new()),
            d_inodes: Mutex::new(lru::LruCache::new(32)),
            fs: Once::new(),
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.get().unwrap().upgrade().unwrap()
    }

    pub fn init(&self, fs: Weak<Ext2Filesystem>) {
        self.fs.call_once(|| fs);

        let fs = self.fs();
        let sb = fs.superblock();

        let mut desc = self.d_desc.write();

        desc.resize_with(sb.group_count(), || {
            disk::blockgroup::BlockGroupDescriptor::default()
        });

        let dev = fs.dev();

        let sector = match sb.block_size() {
            1024 => 4,
            a if a > 1024 => a / 512,
            _ => unreachable!(),
        };

        dev.read(sector, desc.as_mut_slice().to_bytes_mut())
            .expect("Failed to read Block Group Descriptors");
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

        for d in l.iter() {
            println!("{:?}", d);
        }
    }

    pub fn sync(&self, fs: &Ext2Filesystem) {
        let iter = self.d_inodes.lock();

        for (block, e) in iter.iter() {
            let mut el = e.write();

            if el.dirty {
                fs.write_block(*block, el.vec.as_slice().to_bytes());

                el.dirty = false;
            }
        }
    }
}
