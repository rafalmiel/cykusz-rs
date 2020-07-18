use alloc::sync::Arc;

use crate::kernel::device::Result as DevResult;
use crate::kernel::device::{DevError, Device};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;

pub struct DevNode {
    dev: Arc<dyn Device>,
}

impl DevNode {
    pub fn new(devid: usize) -> DevResult<Arc<DevNode>> {
        let dev = crate::kernel::device::devices().read();

        if let Some(d) = dev.get(&devid) {
            Ok(Arc::new(DevNode { dev: d.clone() }))
        } else {
            Err(DevError::DeviceNotFound)
        }
    }
}

impl INode for DevNode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.dev.inode().read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        self.dev.inode().write_at(offset, buf)
    }

    fn poll_listen(&self, listen: bool) -> Result<bool> {
        self.dev.inode().poll_listen(listen)
    }

    fn poll_unlisten(&self) -> Result<()> {
        self.dev.inode().poll_unlisten()
    }
}
