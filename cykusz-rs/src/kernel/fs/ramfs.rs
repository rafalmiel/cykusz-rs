#![allow(unused_variables)]

use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::kernel::fs::devnode::DevNode;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::fs::vfs::{FileType, FsError, Metadata};
use crate::kernel::sync::RwLock;

struct LockedRamINode(RwLock<RamINode>);

enum Content {
    Bytes(Vec<u8>),
    DevNode(Option<Arc<DevNode>>),
}

impl Default for Content {
    fn default() -> Self {
        Content::Bytes(Vec::new())
    }
}

#[derive(Default)]
struct RamINode {
    id: usize,
    typ: FileType,
    parent: Weak<LockedRamINode>,
    this: Weak<LockedRamINode>,
    children: BTreeMap<String, Arc<LockedRamINode>>,
    fs: Weak<RamFS>,
    content: Content,
}

impl INode for LockedRamINode {
    fn metadata(&self) -> Result<Metadata> {
        let i = self.0.read();

        Ok(Metadata {
            id: i.id,
            typ: i.typ,
        })
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

        let inode = this.fs.upgrade().unwrap().alloc_inode(FileType::Dir);

        inode.setup(&this.this, &Arc::downgrade(&inode), &this.fs);

        this.children.insert(String::from(name), inode.clone());

        Ok(inode.clone())
    }

    fn mknode(&self, name: &str, devid: usize) -> Result<Arc<dyn INode>> {
        let mut this = self.0.write();

        if this.children.contains_key(&String::from(name)) {
            return Err(FsError::EntryExists);
        }

        let inode = this.fs.upgrade().unwrap().alloc_inode(FileType::DevNode);
        inode.setup(&this.this, &Arc::downgrade(&inode), &this.fs);

        inode.0.write().content = Content::DevNode(Some(
            DevNode::new(devid).map_err(|e| FsError::EntryNotFound)?,
        ));

        this.children.insert(String::from(name), inode.clone());

        Ok(inode.clone())
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        let i = self.0.read();

        match &i.content {
            Content::Bytes(_) => Err(FsError::NotSupported),
            Content::DevNode(Some(node)) => node.read_at(offset, buf),
            _ => Err(FsError::NotSupported),
        }
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        let i = self.0.read();

        match &i.content {
            Content::Bytes(_) => Err(FsError::NotSupported),
            Content::DevNode(Some(node)) => node.write_at(offset, buf),
            _ => Err(FsError::NotSupported),
        }
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.0.read().fs.upgrade().unwrap().clone()
    }
}

impl LockedRamINode {
    fn setup(&self, parent: &Weak<LockedRamINode>, this: &Weak<LockedRamINode>, fs: &Weak<RamFS>) {
        let mut i = self.0.write();

        i.parent = parent.clone();
        i.this = this.clone();
        i.fs = fs.clone();
        i.id = fs.upgrade().unwrap().alloc_id();
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

        root.setup(
            &Arc::downgrade(&fs.root),
            &Arc::downgrade(&root),
            &Arc::downgrade(&fs),
        );

        return fs;
    }

    fn alloc_inode(&self, typ: FileType) -> Arc<LockedRamINode> {
        let inode = Arc::new(LockedRamINode(RwLock::new(RamINode {
            id: self.alloc_id(),
            typ,
            parent: Weak::default(),
            this: Weak::default(),
            children: BTreeMap::new(),
            fs: Weak::default(),
            content: if typ == FileType::DevNode {
                Content::DevNode(None)
            } else {
                Content::Bytes(Vec::new())
            },
        })));

        inode
    }

    pub fn alloc_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}
