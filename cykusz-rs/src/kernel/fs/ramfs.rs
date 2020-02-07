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
use crate::kernel::fs::vfs::{DirEntry, Result};
use crate::kernel::fs::vfs::{FileType, FsError, Metadata};
use crate::kernel::sync::{Mutex, RwLock};

struct LockedRamINode(RwLock<RamINode>);

enum Content {
    Bytes(Mutex<Vec<u8>>),
    DevNode(Option<Arc<DevNode>>),
    None,
}

impl Default for Content {
    fn default() -> Self {
        Content::Bytes(Mutex::new(Vec::new()))
    }
}

#[derive(Default)]
struct RamINode {
    id: usize,
    name: String,
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
    fn lookup(&self, name: &str) -> Result<DirEntry> {
        let this = self.0.read();
        match name {
            "." => Ok(DirEntry {
                name: this.name.clone(),
                inode: this.this.upgrade().ok_or(FsError::EntryNotFound)?,
            }),
            ".." => {
                let parent = this.parent.upgrade().ok_or(FsError::EntryNotFound)?;
                let parent_locked = parent.0.read();
                Ok(DirEntry {
                    name: parent_locked.name.clone(),
                    inode: parent.clone(),
                })
            }
            _ => {
                let child = this.children.get(name).ok_or(FsError::EntryNotFound)?;

                Ok(DirEntry {
                    name: child.0.read().name.clone(),
                    inode: child.clone(),
                })
            }
        }
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.make_inode(name, FileType::Dir, |_| Ok(()))
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        let i = self.0.read();

        match &i.content {
            Content::Bytes(vec) => {
                let vec = vec.lock();

                if offset >= buf.len() {
                    return Err(FsError::InvalidParam);
                }

                let to_copy = core::cmp::min(buf.len(), vec.len() - offset);

                buf[..to_copy].copy_from_slice(&vec.as_slice()[offset..offset + to_copy]);

                Ok(to_copy)
            }
            Content::DevNode(Some(node)) => {
                let n = node.clone();
                drop(i);

                // read_at may sleep, so drop the lock
                n.read_at(offset, buf)
            }
            _ => Err(FsError::NotSupported),
        }
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        let i = self.0.read();

        match &i.content {
            Content::Bytes(vec) => {
                let mut vec = vec.lock();

                if vec.len() < offset + buf.len() {
                    vec.resize(offset + buf.len(), 0);
                }

                vec.as_mut_slice()[offset..buf.len()].copy_from_slice(buf);

                Ok(buf.len())
            }
            Content::DevNode(Some(node)) => {
                let n = node.clone();
                drop(i);

                // write_at may sleep, so drop the lock
                n.write_at(offset, buf)
            },
            _ => Err(FsError::NotSupported),
        }
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.0.read().fs.upgrade().unwrap().clone()
    }

    fn create(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.make_inode(name, FileType::File, |_| Ok(()))
    }

    fn mknode(&self, name: &str, devid: usize) -> Result<Arc<dyn INode>> {
        self.make_inode(name, FileType::DevNode, |inode| {
            inode.0.write().content = Content::DevNode(Some(
                DevNode::new(devid).map_err(|e| FsError::EntryNotFound)?,
            ));
            Ok(())
        })
    }

    fn truncate(&self) -> Result<()> {
        let node = self.0.write();

        match &node.content {
            Content::Bytes(vec) => {
                let mut v = vec.lock();

                v.clear();

                return Ok(());
            }
            _ => return Ok(()),
        }
    }
}

impl LockedRamINode {
    fn setup(
        &self,
        name: &str,
        parent: &Weak<LockedRamINode>,
        this: &Weak<LockedRamINode>,
        fs: &Weak<RamFS>,
    ) {
        let mut i = self.0.write();

        i.name = String::from(name);
        i.parent = parent.clone();
        i.this = this.clone();
        i.fs = fs.clone();
        i.id = fs.upgrade().unwrap().alloc_id();
    }

    fn make_inode(
        &self,
        name: &str,
        typ: FileType,
        init: impl Fn(&Arc<LockedRamINode>) -> Result<()>,
    ) -> Result<Arc<dyn INode>> {
        let mut this = self.0.write();

        if this.children.contains_key(&String::from(name)) {
            return Err(FsError::EntryExists);
        }

        let inode = this.fs.upgrade().unwrap().alloc_inode(typ);

        inode.setup(name, &this.this, &Arc::downgrade(&inode), &this.fs);

        init(&inode)?;

        this.children.insert(String::from(name), inode.clone());

        let res: Result<Arc<dyn INode>> = Ok(inode.clone());

        res
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
            "/",
            &Arc::downgrade(&fs.root),
            &Arc::downgrade(&root),
            &Arc::downgrade(&fs),
        );

        return fs;
    }

    fn alloc_inode(&self, typ: FileType) -> Arc<LockedRamINode> {
        let inode = Arc::new(LockedRamINode(RwLock::new(RamINode {
            id: self.alloc_id(),
            name: String::new(),
            typ,
            parent: Weak::default(),
            this: Weak::default(),
            children: BTreeMap::new(),
            fs: Weak::default(),
            content: match typ {
                FileType::DevNode => Content::DevNode(None),
                FileType::File => Content::Bytes(Mutex::new(Vec::new())),
                FileType::Dir => Content::None,
            },
        })));

        inode
    }

    pub fn alloc_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}
