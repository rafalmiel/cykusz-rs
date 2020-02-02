use alloc::sync::Arc;

use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::ramfs::RamFS;
use crate::kernel::fs::vfs::Result;

struct DevFSINode{
    inner: Arc<dyn INode>,
}

impl DevFSINode {
    fn lookup_dev(&self, name: &str) -> Option<Arc<dyn INode>>{
        let devs = crate::kernel::device::devices().read();

        if let Some(dev) = devs.values().find(|v| v.name().as_str() == name) {
            return Some(dev.inode())
        }

        None
    }
}

pub struct DevFS {
    ramfs: Arc<RamFS>,
}

impl DevFS {
    pub fn new() -> Arc<DevFS> {
        Arc::new(DevFS {
            ramfs: RamFS::new()
        })
    }
}

impl INode for DevFSINode {
    fn id(&self) -> usize {
        self.inner.id()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>> {
        // Lookup any devices in the root directory of the filesystem,
        // otherwise redirect to ramfs
        if let Some(dev) = self.lookup_dev(name) {
            return Ok(dev);
        }
        self.inner.lookup(name)
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.inner.mkdir(name)
    }

    fn open(&self, name: &str) -> Result<Arc<dyn INode>> {
        self.inner.open(name)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.inner.read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        self.inner.write_at(offset, buf)
    }

    fn close(&self) -> Result<()> {
        self.inner.close()
    }
}

impl Filesystem for DevFS {
    fn root_inode(&self) -> Arc<dyn INode> {
        Arc::new(DevFSINode {
            inner: self.ramfs.root_inode()
        })
    }
}

