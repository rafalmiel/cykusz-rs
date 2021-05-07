use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::OpenFlags;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::utils::buffer::BufferQueue;

pub struct Pipe {
    buf: BufferQueue,

    readers: AtomicUsize,
    writers: AtomicUsize,
}

impl Pipe {
    pub fn new() -> Arc<Pipe> {
        Arc::new(Pipe {
            buf: BufferQueue::new(4096 * 4),

            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
        })
    }

    fn inc_readers(&self) -> usize {
        self.readers.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn inc_writers(&self) -> usize {
        self.writers.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn dec_readers(&self) -> usize {
        self.readers.fetch_sub(1, Ordering::SeqCst) - 1
    }

    fn dec_writers(&self) -> usize {
        self.writers.fetch_sub(1, Ordering::SeqCst) - 1
    }
}

impl INode for Pipe {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buf.read_data(buf)?)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        self.buf.append_data(buf)
    }

    fn open(&self, flags: OpenFlags) -> Result<()> {
        if flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
            self.inc_readers();

            self.buf.set_has_readers(true);
        }

        if flags.intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            self.inc_writers();

            self.buf.set_has_writers(true);
        }

        Ok(())
    }

    fn close(&self, flags: OpenFlags) {
        if flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
            if self.dec_readers() == 0 {
                self.buf.set_has_readers(false);
            }
        }

        if flags.intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            if self.dec_writers() == 0 {
                self.buf.set_has_writers(false);
            }
        }
    }
}
