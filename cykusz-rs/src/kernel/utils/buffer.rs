use crate::kernel::fs::vfs;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel::fs::vfs::FsError;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::{LockApi, Spin, SpinGuard};
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

pub struct BufferQueue {
    buffer: Spin<Buffer>,
    shutting_down: AtomicBool,
    writer_queue: WaitQueue,
    reader_queue: WaitQueue,
    has_writers: AtomicBool,
    has_readers: AtomicBool,
}

impl Default for BufferQueue {
    fn default() -> Self {
        BufferQueue::new(4096, true, true)
    }
}

pub struct Buffer {
    data: Vec<u8>,
    r: usize,
    w: usize,
    full: bool,
    // r == w may indicate both empty and full buffer, full boolean disambiguate that
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer::new(4096)
    }
}

impl BufferQueue {
    pub fn new(init_size: usize, has_readers: bool, has_writers: bool) -> BufferQueue {
        BufferQueue {
            buffer: Spin::new(Buffer::new(init_size)),
            shutting_down: AtomicBool::new(false),
            writer_queue: WaitQueue::new(),
            reader_queue: WaitQueue::new(),
            has_writers: AtomicBool::new(has_writers),
            has_readers: AtomicBool::new(has_readers),
        }
    }

    pub fn new_empty(has_readers: bool, has_writers: bool) -> BufferQueue {
        BufferQueue {
            buffer: Spin::new(Buffer::new_empty()),
            shutting_down: AtomicBool::new(false),
            writer_queue: WaitQueue::new(),
            reader_queue: WaitQueue::new(),
            has_writers: AtomicBool::new(has_writers),
            has_readers: AtomicBool::new(has_readers),
        }
    }

    pub fn set_shutting_down(&self, v: bool) {
        self.shutting_down.store(v, Ordering::SeqCst)
    }

    pub fn shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }

    pub fn init_size(&self, size: usize) {
        self.buffer.lock().init_size(size);
    }

    pub fn set_has_readers(&self, has: bool) {
        self.has_readers.store(has, Ordering::Relaxed);

        if !has {
            self.writer_queue.signal_all(syscall_defs::signal::SIGPIPE);
        } else {
            self.writer_queue.notify_all();
        }
    }

    pub fn set_has_writers(&self, has: bool) {
        self.has_writers.store(has, Ordering::Relaxed);

        self.reader_queue.notify_all();
    }

    pub fn has_readers(&self) -> bool {
        self.has_readers.load(Ordering::Relaxed)
    }

    pub fn has_writers(&self) -> bool {
        self.has_writers.load(Ordering::Relaxed)
    }

    pub fn wait_for_readers(&self) -> SignalResult<()> {
        logln!("waiting for readers {}", self.has_readers());
        self.writer_queue
            .wait_for(WaitQueueFlags::empty(), || self.has_readers())?
            .unwrap();
        logln!("got readers");

        Ok(())
    }

    pub fn wait_for_writers(&self) -> SignalResult<()> {
        logln!("waiting for writers {}", self.has_writers());
        self.reader_queue
            .wait_for(WaitQueueFlags::empty(), || self.has_writers())?
            .unwrap();
        logln!("got writers");

        Ok(())
    }

    pub fn has_data(&self) -> bool {
        self.has_data_locked(&self.buffer.lock())
    }

    fn has_data_locked(&self, lock: &SpinGuard<Buffer>) -> bool {
        lock.has_data() || !self.has_writers()
    }

    pub fn try_append_data(&self, data: &[u8]) -> usize {
        if data.is_empty() {
            return 0;
        }

        let buf = self.buffer.lock();

        self.do_try_append_data(data, buf)
    }

    pub fn try_append_data_irq(&self, data: &[u8]) -> usize {
        if data.is_empty() {
            return 0;
        }

        let buf = self.buffer.lock_irq();

        self.do_try_append_data(data, buf)
    }

    fn do_try_append_data(&self, data: &[u8], mut buf: SpinGuard<Buffer>) -> usize {
        let written = buf.append_data(data);

        drop(buf);

        if written > 0 {
            self.reader_queue.notify_one();
        }

        written
    }

    pub fn append_data_flags(
        &self,
        data: &[u8],
        flags: WaitQueueFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        dbgln!(buffer, "appending data len: {}", data.len());

        let mut buffer = self
            .writer_queue
            .wait_lock_for(flags, &self.buffer, |lck| {
                let _ = &lck;
                !self.has_readers() || lck.available_size() > 0
            })?
            .ok_or(vfs::FsError::WouldBlock)?;

        dbgln!(
            buffer,
            "appending data starting: available size: {}",
            buffer.available_size()
        );

        if !self.has_readers() {
            logln!("no readers......");
            return Err(FsError::Pipe);
        }

        let written = buffer.append_data(data);

        dbgln!(buffer, "data appended {}", written);

        drop(buffer);

        self.reader_queue.notify_all();

        Ok(written)
    }

    pub fn append_data(&self, data: &[u8]) -> crate::kernel::fs::vfs::Result<usize> {
        self.append_data_flags(data, WaitQueueFlags::empty())
    }

    pub fn available_size(&self) -> usize {
        self.buffer.lock().available_size()
    }

    pub fn size(&self) -> usize {
        self.buffer.lock().size()
    }

    pub fn read_data_from(
        &self,
        offset: usize,
        buf: &mut [u8],
        transient: bool,
        wg_flags: WaitQueueFlags,
    ) -> vfs::Result<usize> {
        if offset > 0 && !transient {
            return Ok(0);
        }

        dbgln!(buffer, "read data wait");
        let mut buffer = self
            .reader_queue
            .wait_lock_for(wg_flags, &self.buffer, |l| {
                self.has_data_locked(l) || self.shutting_down() || offset > 0
            })?
            .ok_or(vfs::FsError::WouldBlock)?;

        dbgln!(buffer, "read data starting");
        let read = if transient {
            buffer.read_data_transient_from(offset, buf)
        } else {
            buffer.read_data(buf)
        };

        drop(buffer);

        dbgln!(buffer, "read data done {}", read);

        if !transient && read > 0 {
            self.writer_queue.notify_all();
        }

        Ok(read)
    }

    pub fn read_data(&self, buf: &mut [u8]) -> vfs::Result<usize> {
        self.read_data_from(0, buf, false, WaitQueueFlags::empty())
    }

    pub fn read_data_flags(&self, buf: &mut [u8], wg_flags: WaitQueueFlags) -> vfs::Result<usize> {
        self.read_data_from(0, buf, false, wg_flags)
    }

    pub fn try_read_data_transient(&self, buf: &mut [u8]) -> usize {
        let buffer = self.buffer.lock();

        buffer.read_data_transient_from(0, buf)
    }

    pub fn try_read_data(&self, buf: &mut [u8]) -> usize {
        let mut buffer = self.buffer.lock();

        buffer.read_data(buf)
    }

    pub fn writers_queue(&self) -> &WaitQueue {
        &self.writer_queue
    }

    pub fn readers_queue(&self) -> &WaitQueue {
        &self.reader_queue
    }
}

impl Buffer {
    pub fn new(init_size: usize) -> Buffer {
        let mut buf = Buffer {
            data: Vec::with_capacity(init_size),
            r: 0,
            w: 0,
            full: false,
        };
        buf.data.resize(init_size, 0);

        buf
    }

    pub fn new_empty() -> Buffer {
        Buffer {
            data: Vec::new(),
            r: 0,
            w: 0,
            full: true,
        }
    }

    pub fn init_size(&mut self, size: usize) {
        self.data.resize(size, 0);
        self.full = false;
    }

    pub fn available_size(&self) -> usize {
        if self.full {
            return 0;
        }

        if self.r <= self.w {
            self.data.len() - (self.w - self.r)
        } else {
            self.r - self.w
        }
    }

    pub fn size(&self) -> usize {
        return self.data.len() - self.available_size();
    }

    pub fn append_data(&mut self, data: &[u8]) -> usize {
        dbgln!(buffer, "append data {} {} {}", self.r, self.w, data.len());
        if self.full {
            dbgln!(buffer, "append data buffer full");
            return 0;
        }

        if self.r > self.w {
            let cap = self.r - self.w;
            let to_copy = core::cmp::min(cap, data.len());
            dbgln!(buffer, "append data {}..{}", self.w, self.w + to_copy);
            self.data.as_mut_slice()[self.w..self.w + to_copy].copy_from_slice(&data[..to_copy]);
            self.w = (self.w + to_copy) % self.data.len();
            self.full = self.r == self.w;
            dbgln!(buffer, "append data copied {}", to_copy);
            to_copy
        } else {
            let right = self.data.len() - self.w;
            let to_copy = core::cmp::min(right, data.len());
            dbgln!(buffer, "append data 2 {}..{}", self.w, self.w + to_copy);
            self.data.as_mut_slice()[self.w..self.w + to_copy].copy_from_slice(&data[..to_copy]);
            dbgln!(buffer, "append data done");
            self.w = (self.w + to_copy) % self.data.len();
            self.full = self.r == self.w;
            let written = if to_copy < data.len() {
                dbgln!(buffer, "append data recurse");
                self.append_data(&data[to_copy..])
            } else {
                0
            };
            written + to_copy
        }
    }

    pub fn has_data(&self) -> bool {
        self.r != self.w || self.full
    }

    pub fn read_data(&mut self, buf: &mut [u8]) -> usize {
        if (self.r == self.w && !self.full) || buf.is_empty() {
            return 0;
        }

        if self.w > self.r {
            let cap = self.w - self.r;
            let to_read = core::cmp::min(cap, buf.len());
            buf[..to_read].copy_from_slice(&self.data.as_slice()[self.r..self.r + to_read]);
            self.r += to_read;
            self.full = false;
            to_read
        } else {
            let right = self.data.len() - self.r;
            let to_read = core::cmp::min(right, buf.len());
            buf[..to_read].copy_from_slice(&self.data.as_slice()[self.r..self.r + to_read]);
            self.r = (self.r + to_read) % self.data.len();
            self.full = false;
            let read = if to_read < buf.len() {
                self.read_data(&mut buf[to_read..])
            } else {
                0
            };
            read + to_read
        }
    }

    pub fn mark_as_read(&mut self, amount: usize) -> usize {
        if amount == 0 {
            return amount;
        }

        let avail = self.size();

        if avail < amount {
            self.r = self.w;
            self.full = false;
            return avail;
        }

        self.r = (self.r + amount) % self.data.len();
        self.full = false;

        amount
    }

    pub fn read_data_transient_from(&self, offset: usize, buf: &mut [u8]) -> usize {
        if offset > self.size() {
            return 0;
        }

        let r = (self.r + offset) & self.data.len();

        if (r == self.w && !self.full && offset == 0) || buf.is_empty() {
            return 0;
        }

        if self.w > self.r {
            let cap = self.w - self.r;
            let to_read = core::cmp::min(cap, buf.len());
            buf[..to_read].copy_from_slice(&self.data.as_slice()[self.r..self.r + to_read]);
            to_read
        } else {
            let right = self.data.len() - self.r;
            let to_read = core::cmp::min(right, buf.len());
            buf[..to_read].copy_from_slice(&self.data.as_slice()[self.r..self.r + to_read]);
            let read = if to_read < buf.len() {
                self.read_data_transient_from(to_read, &mut buf[to_read..])
            } else {
                0
            };
            read + to_read
        }
    }
}
