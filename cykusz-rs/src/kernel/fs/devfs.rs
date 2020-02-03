use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::kernel::device::{Device, DeviceListener, register_device_listener};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::ramfs::RamFS;
use crate::kernel::fs::vfs::{FileType, Metadata, Result};
use crate::kernel::sync::RwLock;

struct DevFSINode {
    name: String,
    meta: Option<Metadata>,
    inner: Arc<dyn INode>,
    fs: Weak<DevFS>,
    parent: Weak<DevFSINode>,
    self_ref: Weak<DevFSINode>,
    devices: Option<RwLock<Vec<Arc<DevFSINode>>>>,
}

impl DevFSINode {
    fn lookup_dev(&self, name: &str) -> Option<Arc<dyn INode>> {
        let devs = self.devices.read();

        if let Some(dev) = devs.iter().find(|v| v.name.as_str() == name) {
            return Some(dev.clone());
        }

        None
    }

    fn add_device(&self, dev: &Arc<dyn Device>) {
        println!("Device {} added", dev.name());
        let mut devs = self.devices.write();

        devs.push(DevFSINode {
            name: dev.name(),
            meta: Some(Metadata {
                id: self.ramfs.alloc_id(),
                typ: FileType::File,
            }),
            inner: dev.inode(),
            fs: self.fs.clone(),
            parent: Weak::default(),
            self_ref: Weak::default(),
            devices: None,
        }.wrap(true));
    }

    fn wrap(self, parent: bool) -> Arc<Self> {
        let node = Arc::new(self);
        let weak = Arc::downgrade(&node);
        let ptr = Arc::into_raw(node) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            if parent {
                (*ptr).parent = (*ptr).self_ref.clone();
            }
            Arc::from_raw(ptr)
        }
    }
}

pub struct DevFS {
    ramfs: Arc<RamFS>,
    root: Arc<DevFSINode>,
    self_ref: Weak<Self>,
}

impl DeviceListener for DevFS {
    fn device_added(&self, dev: &Arc<dyn Device>) {
        println!("Device {} added", dev.name());
        self.root.add_device(dev);
    }
}

impl DevFS {
    fn wrap(self) -> Arc<Self> {
        let node = Arc::new(self);
        let weak = Arc::downgrade(&node);
        let ptr = Arc::into_raw(node) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
        }
    }

    pub fn new() -> Arc<DevFS> {
        let dev = DevFS {
            ramfs: RamFS::new(),
            self_ref: Weak::default(),
            root:
            DevFSINode {
                name: String::from(""),
                meta: None,
                inner: self.ramfs.root_inode(),
                fs: self.self_ref.clone(),
                parent: Weak::default(),
                self_ref: Weak::default(),
            }.wrap(true)
        }.wrap();

        register_device_listener(dev.clone());

        dev
    }
}

impl INode for DevFSINode {
    fn metadata(&self) -> Result<Metadata> {
        if let Some(meta) = &self.meta {
            return Ok(*meta)
        } else {
            self.inner.metadata()
        }
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>> {
        // Lookup any devices in the root directory of the filesystem,
        // otherwise redirect to ramfs
        println!("Lookup: {} {}", name, self.id().unwrap());
        match name {
            "" | "." => Ok(self.self_ref.upgrade().unwrap()),
            ".." => Ok(self.parent.upgrade().unwrap()),
            _ => {
                if let Some(dev) = self.lookup_dev(name) {
                    return Ok(dev);
                }

                Ok(DevFSINode {
                    name: String::from(""),
                    meta: None,
                    inner: self.inner.lookup(name)?,
                    fs: self.fs.clone(),
                    parent: self.self_ref.clone(),
                    self_ref: Weak::default(),
                }.wrap(false))
            }
        }
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.inner.mkdir(name)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.inner.read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        self.inner.write_at(offset, buf)
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        self.fs.upgrade().unwrap().clone()
    }
}

impl Filesystem for DevFS {
    fn root_inode(&self) -> Arc<dyn INode> {
        DevFSINode {
            name: String::from(""),
            meta: None,
            inner: self.ramfs.root_inode(),
            fs: self.self_ref.clone(),
            parent: Weak::default(),
            self_ref: Weak::default(),
        }.wrap(true)
    }
}
