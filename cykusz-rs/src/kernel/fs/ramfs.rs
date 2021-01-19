#![allow(unused_variables)]

use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use syscall_defs::FileType;

use crate::kernel::device::Device;
use crate::kernel::fs::devnode::DevNode;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::fs::vfs::{FsError, Metadata};
use crate::kernel::sync::{RwSpin, Spin};
use crate::kernel::syscall::sys::PollTable;

struct LockedRamINode(RwSpin<RamINode>);

enum Content {
    Bytes(Spin<Vec<u8>>),
    DevNode(Option<Arc<DevNode>>),
    None,
}

impl Default for Content {
    fn default() -> Self {
        Content::Bytes(Spin::new(Vec::new()))
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

    fn lookup(
        &self,
        parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        name: &str,
    ) -> Result<Arc<super::dirent::DirEntry>> {
        let this = self.0.read();

        let child = this.children.get(name).ok_or(FsError::EntryNotFound)?;

        Ok(super::dirent::DirEntry::new(
            parent.clone(),
            child.clone(),
            String::from(name),
        ))
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
            }
            _ => Err(FsError::NotSupported),
        }
    }

    fn poll(&self, ptable: Option<&mut PollTable>) -> Result<bool> {
        let i = self.0.read();

        match &i.content {
            Content::DevNode(Some(n)) => {
                let n = n.clone();
                drop(i);

                n.poll(ptable)
            }
            _ => Err(FsError::NotSupported),
        }
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.0.read().fs.upgrade().unwrap().clone()
    }

    fn create(
        &self,
        parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        name: &str,
    ) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
        Ok(super::dirent::DirEntry::new(
            parent.clone(),
            self.make_inode(name, FileType::File, |_| Ok(()))?,
            String::from(name),
        ))
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

    fn dir_ent(
        &self,
        parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        idx: usize,
    ) -> Result<Option<Arc<crate::kernel::fs::dirent::DirEntry>>> {
        let d = self.0.read();

        if d.typ != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let fs = d.fs.upgrade().unwrap();

        let dir = match idx {
            0 => Some(crate::kernel::fs::dirent::DirEntry::new(
                parent,
                d.this.upgrade().unwrap(),
                String::from("."),
            )),
            1 => Some(crate::kernel::fs::dirent::DirEntry::new(
                parent,
                d.parent.upgrade().unwrap(),
                String::from(".."),
            )),
            idx => {
                if let Some(e) = d.children.iter().nth(idx - 2) {
                    Some(crate::kernel::fs::dirent::DirEntry::new(
                        parent,
                        e.1.clone(),
                        e.0.clone(),
                    ))
                } else {
                    None
                }
            }
        };

        Ok(dir)
    }

    fn device(&self) -> Result<Arc<dyn Device>> {
        if let Content::DevNode(Some(d)) = &self.0.read().content {
            Ok(d.device())
        } else {
            Err(FsError::EntryNotFound)
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

        if this.children.contains_key(&String::from(name)) || ["", ".", ".."].contains(&name) {
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
    root_dentry: Arc<super::dirent::DirEntry>,
    next_id: AtomicUsize,
}

impl Filesystem for RamFS {
    fn root_dentry(&self) -> Arc<super::dirent::DirEntry> {
        self.root_dentry.clone()
    }

    fn name(&self) -> &'static str {
        "ramfs"
    }
}

impl RamFS {
    pub fn new() -> Arc<RamFS> {
        let root = Arc::new(LockedRamINode(RwSpin::new(RamINode::default())));

        let root_de = super::dirent::DirEntry::new_root(root.clone(), String::from("/"));

        let fs = Arc::new(RamFS {
            root: root.clone(),
            root_dentry: root_de.clone(),
            next_id: AtomicUsize::new(1),
        });

        let cpy: Arc<dyn Filesystem> = fs.clone();

        root_de.init_fs(Arc::downgrade(&cpy));

        root.setup(
            "/",
            &Arc::downgrade(&fs.root),
            &Arc::downgrade(&root),
            &Arc::downgrade(&fs),
        );
        root.0.write().typ = FileType::Dir;

        return fs;
    }

    fn alloc_inode(&self, typ: FileType) -> Arc<LockedRamINode> {
        let inode = Arc::new(LockedRamINode(RwSpin::new(RamINode {
            id: self.alloc_id(),
            name: String::new(),
            typ,
            parent: Weak::default(),
            this: Weak::default(),
            children: BTreeMap::new(),
            fs: Weak::default(),
            content: match typ {
                FileType::DevNode => Content::DevNode(None),
                FileType::File => Content::Bytes(Spin::new(Vec::new())),
                FileType::Dir => Content::None,
                FileType::Symlink => Content::None,
            },
        })));

        inode
    }

    pub fn alloc_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}
