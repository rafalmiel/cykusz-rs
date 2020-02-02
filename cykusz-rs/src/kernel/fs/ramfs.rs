#![allow(unused_variables)]

use alloc::collections::btree_map::{BTreeMap};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::fs::vfs::Result;
use crate::kernel::sync::RwLock;

struct LockedRamINode(RwLock<RamINode>);

#[derive(Default)]
struct RamINode {
    id: usize,
    parent: Weak<LockedRamINode>,
    this: Weak<LockedRamINode>,
    children: BTreeMap<String, Arc<LockedRamINode>>,
    fs: Weak<RamFS>,
}

impl INode for LockedRamINode {
    fn id(&self) -> usize {
        self.0.read().id
    }
    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>> {
        let this = self.0.read();
        match name {
            "." => Ok(this.this.upgrade().ok_or(FsError::EntryNotFound)?),
            ".." => Ok(this.parent.upgrade().ok_or(FsError::EntryNotFound)?),
            _ => {
                let child = this.children.get(name).ok_or(FsError::EntryNotFound)?;

                Ok(child.clone())
            }
        }
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        let mut this = self.0.write();

        if this.children.contains_key(&String::from(name)) {
            return Err(FsError::EntryExists);
        }

        let inode = this.fs.upgrade().unwrap().alloc_inode();

        inode.setup(&this.this,
                    &Arc::downgrade(&inode),
                    &this.fs);

        this.children.insert(String::from(name), inode.clone());

        Ok(inode.clone())
    }

    fn open(&self, name: &str) -> Result<Arc<dyn INode>> {
        unimplemented!()
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        unimplemented!()
    }

    fn close(&self) -> Result<()> {
        unimplemented!()
    }
}

impl LockedRamINode {
    fn setup(&self, parent: &Weak<LockedRamINode>, this: &Weak<LockedRamINode>, fs: &Weak<RamFS>) {
        let mut i = self.0.write();

        i.parent = parent.clone();
        i.this = this.clone();
        i.fs = fs.clone();
    }
}

pub struct RamFS {
    root: Arc<LockedRamINode>,
    next_id: AtomicUsize,
}

impl Filesystem for RamFS {
    fn root_inode(&self) -> Arc<dyn INode> {
        self.root.clone()
    }
}

impl RamFS {
    pub fn new() -> Arc<RamFS> {
        let root = Arc::new(LockedRamINode(RwLock::new(RamINode::default())));

        let fs = Arc::new(RamFS {
            root: root.clone(),
            next_id: AtomicUsize::new(1),
        });

        root.setup(&Arc::downgrade(&fs.root),
                   &Arc::downgrade(&root),
                   &Arc::downgrade(&fs));

        return fs;
    }

    fn alloc_inode(&self) -> Arc<LockedRamINode> {
        let inode = Arc::new(LockedRamINode(RwLock::new(RamINode {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            parent: Weak::default(),
            this: Weak::default(),
            children: BTreeMap::new(),
            fs: Weak::default(),
        })));

        inode
    }
}
