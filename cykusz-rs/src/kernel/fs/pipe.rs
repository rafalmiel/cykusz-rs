use alloc::sync::{Arc, Weak};
use core::sync::atomic::{AtomicUsize, Ordering};

use hashbrown::HashMap;
use spin::Once;

use syscall_defs::poll::PollEventFlags;
use syscall_defs::OpenFlags;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::{Mutex, MutexGuard};
use crate::kernel::utils::buffer::BufferQueue;

pub struct Pipe {
    buf: BufferQueue,
    sref: Weak<Pipe>,
    key: Option<(usize, usize)>,

    readers: AtomicUsize,
    writers: AtomicUsize,
}

impl Pipe {
    pub fn new(key: Option<(usize, usize)>) -> Arc<Pipe> {
        logln4!("Created PIPE");
        Arc::new_cyclic(|me| Pipe {
            buf: BufferQueue::new(4096 * 4, false, false),
            sref: me.clone(),
            key,

            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
        })
    }

    pub fn sref(&self) -> Arc<Pipe> {
        self.sref.upgrade().unwrap()
    }

    fn key(&self) -> Option<(usize, usize)> {
        self.key
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
        logln!("pipe open: {:?}", flags);
        if flags.contains(OpenFlags::RDWR) {
            return Err(FsError::InvalidParam);
        }

        if flags.contains(OpenFlags::RDONLY) {
            self.inc_readers();

            self.buf.set_has_readers(true);

            if !flags.contains(OpenFlags::NONBLOCK) {
                self.buf.wait_for_writers()?;
            }
        }

        if flags.contains(OpenFlags::WRONLY) {
            if flags.contains(OpenFlags::NONBLOCK) && !self.has_readers() {
                return Err(FsError::NoSuchDevice);
            }

            self.inc_writers();

            self.buf.set_has_writers(true);

            if !flags.contains(OpenFlags::NONBLOCK) {
                self.buf.wait_for_readers()?;
            }
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

        if !self.has_readers() && !self.has_writers() {
            pipes().remove(&self.sref());
        }
    }
}

pub struct PipeMap {
    map: HashMap<(usize, usize), Arc<Pipe>>,
}

impl PipeMap {
    pub fn new() -> PipeMap {
        PipeMap {
            map: HashMap::new(),
        }
    }

    fn get_key(inode: &Arc<dyn INode>) -> Option<(usize, usize)> {
        Some((
            Arc::as_ptr(&inode.fs()?.upgrade()?.device()) as *const () as usize,
            inode.id().unwrap(),
        ))
    }

    pub fn get_or_insert(&mut self, inode: &Arc<dyn INode>) -> Option<Arc<Pipe>> {
        let key = Self::get_key(inode)?;

        match self.map.try_insert(key, Pipe::new(Some(key))) {
            Ok(v) => {
                logln!("getting new pipe -> created: {:?}", key);
                Some(v.clone())
            }
            Err(e) => {
                logln!("getting new pipe -> returned: {:?}", key);
                Some(e.entry.get().clone())
            }
        }
    }

    pub fn remove(&mut self, inode: &Arc<Pipe>) {
        if let Some(k) = inode.key() {
            logln!("remove pipe {:?}", k);
            self.map.remove(&k);
        }
    }
}

static PIPES: Once<Mutex<PipeMap>> = Once::new();

pub fn init() {
    PIPES.call_once(|| Mutex::new(PipeMap::new()));
}

pub fn pipes<'a>() -> MutexGuard<'a, PipeMap> {
    unsafe { PIPES.get_unchecked().lock() }
}
