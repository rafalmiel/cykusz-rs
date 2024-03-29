use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::poll::PollEventFlags;
use syscall_defs::stat::Stat;
use syscall_defs::OpenFlags;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::Result;
use crate::kernel::utils::buffer::BufferQueue;

pub struct Pipe {
    buf: BufferQueue,

    readers: AtomicUsize,
    writers: AtomicUsize,
}

impl Pipe {
    pub fn new() -> Arc<Pipe> {
        logln4!("Created PIPE");
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

    fn has_writers(&self) -> bool {
        self.writers.load(Ordering::Relaxed) > 0
    }

    fn has_readers(&self) -> bool {
        self.readers.load(Ordering::Relaxed) > 0
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        logln4!("Dopped PIPE");
    }
}

impl INode for Pipe {
    fn stat(&self) -> Result<Stat> {
        let mut stat = Stat::default();

        stat.st_mode.insert(syscall_defs::stat::Mode::IFIFO);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXU);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXG);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXO);

        Ok(stat)
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buf.read_data(buf)?)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        self.buf.append_data(buf)
    }

    fn poll(
        &self,
        poll_table: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        let mut res_flags = PollEventFlags::empty();
        if flags.contains(PollEventFlags::READ) {
            if self.buf.has_data() {
                res_flags.insert(PollEventFlags::READ);
            }

            if !self.has_writers() {
                res_flags.insert(PollEventFlags::HUP);
            }
        }
        if flags.contains(PollEventFlags::WRITE) {
            if self.buf.available_size() > 0 {
                res_flags.insert(PollEventFlags::WRITE);
            }

            if !self.has_readers() {
                res_flags.insert(PollEventFlags::ERR);
            }
        }

        if let Some(p) = poll_table {
            if flags.contains(PollEventFlags::READ) {
                p.listen(&self.buf.readers_queue());
            }
            if flags.contains(PollEventFlags::WRITE) {
                p.listen(&self.buf.writers_queue());
            }
        }

        Ok(res_flags)
    }

    fn open(&self, flags: OpenFlags) -> Result<()> {
        if flags.contains(OpenFlags::RDONLY) {
            self.inc_readers();

            self.buf.set_has_readers(true);
        }

        if flags.contains(OpenFlags::WRONLY) {
            self.inc_writers();

            self.buf.set_has_writers(true);
        }

        Ok(())
    }

    fn close(&self, flags: OpenFlags) {
        if flags.contains(OpenFlags::RDONLY) {
            if self.dec_readers() == 0 {
                self.buf.set_has_readers(false);
            }
        }

        if flags.contains(OpenFlags::WRONLY) {
            if self.dec_writers() == 0 {
                self.buf.set_has_writers(false);
            }
        }
    }
}
