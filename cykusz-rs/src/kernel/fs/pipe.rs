use crate::kernel::utils::buffer::BufferQueue;

use alloc::sync::Arc;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;

pub struct Pipe {
    buf: BufferQueue
}

impl Pipe {
    pub fn new() -> Arc<Pipe> {
        Arc::new(Pipe {
            buf: BufferQueue::new(4096*4),
        })
    }
}

impl INode for Pipe {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buf.read_data(buf)?)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        Ok(self.buf.append_data(buf)?)
    }
}
