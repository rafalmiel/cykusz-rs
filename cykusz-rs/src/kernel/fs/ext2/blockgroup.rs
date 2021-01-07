use super::disk;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::sync::{RwSpin, RwSpinReadGuard};
use crate::kernel::utils::slice::ToBytesMut;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use spin::Once;

use alloc::collections::BTreeMap;

pub struct INodeVec(pub Vec<disk::inode::INode>);

pub struct INodeGroup {
    inodes: RwSpin<INodeVec>,
}

impl INodeVec {
    pub fn get(&self, id: usize) -> &disk::inode::INode {
        &self.0[(id - 1) % self.0.len()]
    }
}

impl INodeGroup {
    pub fn read(&self) -> RwSpinReadGuard<INodeVec> {
        self.inodes.read()
    }
}

pub struct BlockGroupDescriptors {
    d_desc: RwSpin<Vec<disk::blockgroup::BlockGroupDescriptor>>,
    d_inodes: RwSpin<BTreeMap<usize, Arc<INodeGroup>>>,
    fs: Once<Weak<super::Ext2Filesystem>>,
}

impl BlockGroupDescriptors {
    pub fn new() -> BlockGroupDescriptors {
        BlockGroupDescriptors {
            d_desc: RwSpin::new(Vec::new()),
            d_inodes: RwSpin::new(BTreeMap::new()),
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

        let inodes = self.d_inodes.read_upgradeable();
        if let Some(e) = inodes.get(&block) {
            e.clone()
        } else {
            let fs = self.fs();
            let sb = fs.superblock().read_inner();

            //load
            let mut vec = Vec::<disk::inode::INode>::new();
            vec.resize(sb.inodes_per_block(), disk::inode::INode::default());

            let spb = sb.sectors_per_block();

            fs.dev()
                .read(block * spb, vec.as_mut_slice().to_bytes_mut());

            let mut inodes_w = inodes.upgrade();

            let res = Arc::new(INodeGroup {
                inodes: RwSpin::new(INodeVec(vec)),
            });

            inodes_w.insert(block, res.clone());

            res
        }
    }
}
