#![allow(unused_variables)]

use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use syscall_defs::{FileType, OpenFlags};
use syscall_defs::poll::PollEventFlags;

use crate::kernel::device::{alloc_id, Device};
use crate::kernel::fs::devnode::DevNode;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::ext2::FsDevice;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemInt, INodeItemStruct};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::MappedAccess;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{FsError, Metadata};
use crate::kernel::fs::vfs::Result;
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::sync::{RwSpin, Spin};
use crate::kernel::utils::types::CeilDiv;

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
    #[allow(unused)]
    name: String,
    typ: FileType,
    parent: Weak<INodeItemInt>,
    this: Weak<INodeItemInt>,
    children: BTreeMap<String, INodeItem>,
    fs: Weak<RamFS>,
    content: Content,
}

impl INode for LockedRamINode {
    fn metadata(&self) -> Result<Metadata> {
        let i = self.0.read();

        let size = match &i.content {
            Content::Bytes(b) => b.lock().len(),
            _ => 0,
        };

        let res = Ok(Metadata {
            id: i.id,
            typ: i.typ,
            size,
        });

        res
    }

    fn stat(&self) -> Result<syscall_defs::stat::Stat> {
        let mut stat = syscall_defs::stat::Stat::default();

        stat.st_ino = self.id()? as u64;
        stat.st_dev = 0;

        let content = self.0.read();
        if let Content::Bytes(b) = &content.content {
            let bytes = b.lock();

            stat.st_nlink = 1;
            stat.st_blksize = PAGE_SIZE as u64;
            stat.st_blocks = bytes.len().ceil_div(PAGE_SIZE) as u64;
            stat.st_size = bytes.len() as i64;
        }

        let ftype = content.typ;
        if ftype == FileType::File {
            stat.st_mode.insert(syscall_defs::stat::Mode::IFREG);
        } else if ftype == FileType::Dir {
            stat.st_mode.insert(syscall_defs::stat::Mode::IFDIR);
        } else if ftype == FileType::Symlink {
            stat.st_mode.insert(syscall_defs::stat::Mode::IFLNK);
        } else {
            stat.st_mode.insert(syscall_defs::stat::Mode::IFCHR);
        }

        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXU);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXG);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXO);

        Ok(stat)
    }

    fn lookup(&self, parent: DirEntryItem, name: &str) -> Result<DirEntryItem> {
        let this = self.0.read();

        let child = this.children.get(name).ok_or(FsError::EntryNotFound)?;

        Ok(super::dirent::DirEntry::new(
            parent.clone(),
            child.clone(),
            String::from(name),
        ))
    }

    fn mkdir(&self, name: &str) -> Result<INodeItem> {
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

                vec.as_mut_slice()[offset..offset + buf.len()].copy_from_slice(buf);

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

    fn poll(
        &self,
        ptable: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        let i = self.0.read();

        match &i.content {
            Content::DevNode(Some(n)) => {
                let n = n.clone();
                drop(i);

                n.poll(ptable, flags)
            }
            _ => Err(FsError::NotSupported),
        }
    }

    fn fs(&self) -> Option<Weak<dyn Filesystem>> {
        Some(self.0.read().fs.clone())
    }

    fn create(&self, parent: DirEntryItem, name: &str, ftype: FileType) -> Result<DirEntryItem> {
        Ok(super::dirent::DirEntry::new(
            parent.clone(),
            self.make_inode(name, ftype, |_| Ok(()))?,
            String::from(name),
        ))
    }

    fn open(&self, flags: OpenFlags) -> Result<()> {
        if let Content::DevNode(Some(d)) = &self.0.read().content {
            d.open(flags)
        } else {
            Ok(())
        }
    }

    fn mknode(
        &self,
        _parent: DirEntryItem,
        name: &str,
        mode: syscall_defs::stat::Mode,
        devid: usize,
    ) -> Result<INodeItem> {
        self.make_inode(name, mode.into(), |inode| {
            inode.0.write().content = Content::DevNode(Some(
                DevNode::new(devid).map_err(|e| FsError::EntryNotFound)?,
            ));
            Ok(())
        })
    }

    fn truncate(&self, size: usize) -> Result<()> {
        let node = self.0.write();

        match &node.content {
            Content::Bytes(vec) => {
                let mut v = vec.lock();

                v.resize(size, 0);

                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    fn dir_ent(&self, parent: DirEntryItem, idx: usize) -> Result<Option<DirEntryItem>> {
        let d = self.0.read();

        if d.typ != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let fs = d.fs.upgrade().unwrap();

        let dir = match idx {
            0 => Some(crate::kernel::fs::dirent::DirEntry::new(
                parent,
                d.this.upgrade().unwrap().into(),
                String::from("."),
            )),
            1 => Some(crate::kernel::fs::dirent::DirEntry::new(
                parent,
                d.parent.upgrade().unwrap().into(),
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

    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize> {
        let read = self.0.read();
        if let Content::DevNode(Some(d)) = &read.content {
            let dev = d.clone();
            drop(read);
            dev.ioctl(cmd, arg)
        } else {
            Err(FsError::NotSupported)
        }
    }

    fn as_mappable(&self) -> Option<Arc<dyn MappedAccess>> {
        if let Content::DevNode(Some(d)) = &self.0.read().content {
            d.device().inode().as_mappable()
        } else {
            None
        }
    }
}

impl LockedRamINode {
    fn setup(
        &self,
        name: &str,
        parent: &Weak<INodeItemInt>,
        this: &Weak<INodeItemInt>,
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
        init: impl Fn(&LockedRamINode) -> Result<()>,
    ) -> Result<INodeItem> {
        let inode = {
            let mut this = self.0.write();

            if this.children.contains_key(&String::from(name)) || ["", ".", ".."].contains(&name) {
                return Err(FsError::EntryExists);
            }

            let inode = this.fs.upgrade().unwrap().alloc_inode(typ);

            let cache = crate::kernel::fs::icache::cache();

            let inode = cache.make_item_no_cache(INodeItemStruct::from(inode));

            inode
                .as_ramfs_inode()
                .setup(name, &this.this, &Arc::downgrade(&inode), &this.fs);

            init(inode.as_ramfs_inode())?;

            this.children.insert(String::from(name), inode.clone());

            inode
        };

        //crate::kernel::fs::icache::cache().make_cached(&inode);

        Ok(inode)
    }
}

pub struct RamFS {
    root: INodeItem,
    root_dentry: DirEntryItem,
    next_id: AtomicUsize,
    dev: Arc<dyn FsDevice>,
}

impl Filesystem for RamFS {
    fn root_dentry(&self) -> DirEntryItem {
        self.root_dentry.clone()
    }

    fn name(&self) -> &'static str {
        "ramfs"
    }

    fn device(&self) -> Arc<dyn FsDevice> {
        self.dev.clone()
    }
}

struct DummyRamDevice {
    id: usize,
    self_ref: Weak<DummyRamDevice>,
}

impl DummyRamDevice {
    fn new() -> Arc<DummyRamDevice> {
        Arc::new_cyclic(|me| DummyRamDevice {
            id: alloc_id(),
            self_ref: me.clone(),
        })
    }
}

impl INode for DummyRamDevice {}

impl Device for DummyRamDevice {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        alloc::format!("ram_dev {}", self.id)
    }

    fn inode(&self) -> Arc<dyn INode> {
        return self.self_ref.upgrade().unwrap();
    }
}

impl FsDevice for DummyRamDevice {}

impl RamFS {
    pub fn new(dev: Option<Arc<dyn FsDevice>>) -> Arc<RamFS> {
        let cache = crate::kernel::fs::icache::cache();
        let root = Arc::new(LockedRamINode(RwSpin::new(RamINode::default())));
        let root = cache.make_item_no_cache(INodeItemStruct::from(root));

        let root_de = super::dirent::DirEntry::new_root(root.clone(), String::from("/"));

        let dummy = DummyRamDevice::new();

        let fs = Arc::new(RamFS {
            root: root.clone(),
            root_dentry: root_de.clone(),
            next_id: AtomicUsize::new(1),
            dev: dev.unwrap_or(dummy.clone()),
        });

        let cpy: Arc<dyn Filesystem> = fs.clone();

        root_de.init_fs(Arc::downgrade(&cpy));

        root.as_ramfs_inode().setup(
            "/",
            &Arc::downgrade(&fs.root),
            &Arc::downgrade(&root),
            &Arc::downgrade(&fs),
        );
        root.as_ramfs_inode().0.write().typ = FileType::Dir;

        return fs;
    }

    fn alloc_inode(&self, typ: FileType) -> Arc<LockedRamINode> {
        Arc::new(LockedRamINode(RwSpin::new(RamINode {
            id: self.alloc_id(),
            name: String::new(),
            typ,
            parent: Weak::default(),
            this: Weak::default(),
            children: BTreeMap::new(),
            fs: Weak::default(),
            content: match typ {
                FileType::Char => Content::DevNode(None),
                FileType::Block => Content::DevNode(None),
                FileType::File => Content::Bytes(Spin::new(Vec::new())),
                FileType::Dir => Content::None,
                FileType::Symlink => Content::None,
                _ => Content::None,
            },
        })))
    }

    pub fn alloc_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl INodeItemStruct {
    pub(in crate::kernel::fs::ramfs) fn as_ramfs_inode(&self) -> &LockedRamINode {
        self.as_impl::<LockedRamINode>()
    }
}
