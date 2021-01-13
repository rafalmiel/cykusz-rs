use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};

use syscall_defs::FileType;

use crate::kernel::device::Device;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{DirEntIter, DirEntry, FsError, Metadata, Result};
use crate::kernel::sync::RwSpin;
use crate::kernel::syscall::sys::PollTable;

#[allow(dead_code)]
pub struct MountFS {
    fs: Arc<dyn Filesystem>,
    mounts: RwSpin<BTreeMap<usize, Arc<MountFS>>>,
    self_mount: Option<Arc<MNode>>,
    self_ref: Weak<MountFS>,
}

impl MountFS {
    pub fn new(fs: Arc<dyn Filesystem>) -> Arc<MountFS> {
        Arc::new_cyclic(|me| MountFS {
            fs,
            mounts: RwSpin::new(BTreeMap::new()),
            self_mount: None,
            self_ref: me.clone(),
        })
    }

    pub fn root_inode(&self) -> Arc<MNode> {
        Arc::new_cyclic(|me| MNode {
            name: String::from("/"),
            inode: self.fs.root_inode(),
            vfs: self.self_ref.upgrade().unwrap(),
            self_ref: me.clone(),
        })
    }
}

impl Filesystem for MountFS {
    fn root_inode(&self) -> Arc<dyn INode> {
        self.root_inode()
    }
}

pub struct MNode {
    name: String,
    // The inner INode
    inode: Arc<dyn INode>,
    // Associated MountFilesystem
    vfs: Arc<MountFS>,
    // Weak reference to self
    self_ref: Weak<MNode>,
}

impl MNode {
    fn wrap(mut self) -> Arc<MNode> {
        Arc::new_cyclic(|me| {
            self.self_ref = me.clone();
            self
        })
    }

    pub fn mount(&self, fs: Arc<dyn Filesystem>) -> Result<Arc<MountFS>> {
        if self.inode.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let node = if self.is_root() {
            self.vfs.self_mount.as_ref().unwrap()
        } else {
            self
        };

        let fs = Arc::new_cyclic(|me| MountFS {
            fs,
            mounts: RwSpin::new(BTreeMap::new()),
            self_mount: Some(node.self_ref.upgrade().unwrap()),
            self_ref: me.clone(),
        });

        let mut mounts = node.vfs.mounts.write();

        if mounts.contains_key(&node.id()?) {
            return Err(FsError::EntryExists);
        } else {
            mounts.insert(node.id()?, fs.clone());
        }

        Ok(fs)
    }

    pub fn umount(&self) -> Result<()> {
        if !self.is_root() {
            return Err(FsError::NotSupported);
        }

        if let Some(node) = self.vfs.self_mount.as_ref() {
            let mut mounts = node.vfs.mounts.write();

            let node_id = node.id()?;

            if let Some(mount) = mounts.get(&node_id) {
                if Arc::strong_count(&mount.fs) > 1 {
                    println!("[ MountFS ] Warn: Trying to unmount busy filesystem");
                }
            }

            mounts.remove(&node_id);

            Ok(())
        } else {
            Err(FsError::NotSupported)
        }
    }

    fn covering_node(&self) -> Arc<MNode> {
        let id = self.id();

        if id.is_err() {
            return self.self_ref.upgrade().unwrap();
        }

        match self.vfs.mounts.read().get(&id.unwrap()) {
            Some(node) => node.root_inode(),
            None => self.self_ref.upgrade().unwrap(),
        }
    }

    fn is_root(&self) -> bool {
        let id1 = self.inode.fs().root_inode().id().unwrap();
        let id2 = self.inode.id().unwrap();
        let is = id1 == id2;
        is
    }

    pub fn lookup(&self, name: &str) -> Result<DirEntry> {
        match name {
            "" | "." => Ok(DirEntry {
                name: self.name.clone(),
                inode: self.self_ref.upgrade().unwrap(),
            }),
            ".." => {
                if self.is_root() {
                    match &self.vfs.self_mount {
                        Some(inode) => inode.lookup(".."),
                        None => Ok(DirEntry {
                            name: self.name.clone(),
                            inode: self.self_ref.upgrade().unwrap(),
                        }),
                    }
                } else {
                    let mnode = MNode {
                        name: String::from(name),
                        inode: self.inode.lookup(name)?.inode,
                        vfs: self.vfs.clone(),
                        self_ref: Weak::default(),
                    }
                    .wrap();

                    Ok(DirEntry {
                        name: mnode.name.clone(),
                        inode: mnode,
                    })
                }
            }
            _ => {
                let mnode = MNode {
                    name: String::from(name),
                    inode: self.covering_node().inode.lookup(name)?.inode,
                    vfs: self.vfs.clone(),
                    self_ref: Weak::default(),
                }
                .wrap()
                .covering_node();

                Ok(DirEntry {
                    name: mnode.name.clone(),
                    inode: mnode,
                })
            }
        }
    }

    pub fn mkdir(&self, name: &str) -> Result<Arc<MNode>> {
        Ok(MNode {
            name: String::from(name),
            inode: self.inode.mkdir(name)?,
            vfs: self.vfs.clone(),
            self_ref: Weak::default(),
        }
        .wrap())
    }

    pub fn self_inode(&self) -> Arc<dyn INode> {
        self.self_ref.upgrade().unwrap()
    }
}

impl INode for MNode {
    fn metadata(&self) -> Result<Metadata> {
        self.inode.metadata()
    }

    fn lookup(&self, name: &str) -> Result<DirEntry> {
        Ok(self.lookup(name)?)
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        Ok(self.mkdir(name)?)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.inode.read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        self.inode.write_at(offset, buf)
    }

    fn poll(&self, ptable: Option<&mut PollTable>) -> Result<bool> {
        self.inode.poll(ptable)
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.inode.fs()
    }

    fn create(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.inode.create(name)
    }

    fn mknode(&self, name: &str, devid: usize) -> Result<Arc<dyn INode>> {
        self.inode.mknode(name, devid)
    }

    fn truncate(&self) -> Result<()> {
        self.inode.truncate()
    }

    fn dir_ent(&self, idx: usize) -> Result<Option<DirEntry>> {
        self.inode.dir_ent(idx)
    }

    fn dir_iter(&self) -> Option<Arc<dyn DirEntIter>> {
        self.inode.dir_iter()
    }

    fn mount(&self, fs: Arc<dyn Filesystem>) -> Result<Arc<dyn Filesystem>> {
        if let Ok(res) = self.mount(fs) {
            Ok(res)
        } else {
            Err(FsError::NotDir)
        }
    }

    fn umount(&self) -> Result<()> {
        self.umount()
    }

    fn device(&self) -> Result<Arc<dyn Device>> {
        self.inode.device()
    }
}
