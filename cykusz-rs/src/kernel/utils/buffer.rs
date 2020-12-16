use alloc::vec::Vec;

use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct BufferQueue {
    buffer: Spin<Buffer>,
    wait_queue: WaitQueue,
}

impl Default for BufferQueue {
    fn default() -> Self {
        BufferQueue::new(4096)
    }
}

pub struct Buffer {
    data: Vec<u8>,
    r: usize,
    w: usize,
    full: bool, // r == w may indicate both empty and full buffer, full boolean disambiguate that
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer::new(4096)
    }
}

impl BufferQueue {
    pub fn new(init_size: usize) -> BufferQueue {
        BufferQueue {
            buffer: Spin::new(Buffer::new(init_size)),
            wait_queue: WaitQueue::new(),
        }
    }

    pub fn listen(&self) {
        self.wait_queue.add_task(current_task());
    }

    pub fn unlisten(&self) {
        self.wait_queue.remove_task(current_task());
    }

    pub fn has_data(&self) -> bool {
        self.buffer.lock().has_data()
    }

    pub fn append_data(&self, data: &[u8]) -> usize {
        if data.is_empty() {
            return 0;
        }

        let mut buf = self.buffer.lock();

        let written = buf.append_data(data);

        if written > 0 {
            self.wait_queue.notify_one();
        }

        written
    }

    pub fn available_size(&self) -> usize {
        let a = self.buffer.lock().available_size();

        //println!(" {}", a);

        a
    }

    pub fn read_data(&self, buf: &mut [u8]) -> usize {
        let mut buffer = self.buffer.lock();

        let task = current_task();

        self.wait_queue.add_task(task.clone());

        while !buffer.has_data() {
            WaitQueue::wait_lock(buffer);

            buffer = self.buffer.lock();
        }

        self.wait_queue.remove_task(task);

        buffer.read_data(buf)
    }

    pub fn try_read_data_transient(&self, buf: &mut [u8]) -> usize {
        let buffer = self.buffer.lock();

        buffer.read_data_transient(buf)
    }

    pub fn try_read_data(&self, buf: &mut [u8]) -> usize {
        let mut buffer = self.buffer.lock();

        buffer.read_data(buf)
    }

    pub fn wait_queue(&self) -> &WaitQueue {
        &self.wait_queue
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

    pub fn available_size(&self) -> usize {
        if self.full {
            return 0;
        }

        return if self.r <= self.w {
            self.data.len() - (self.w - self.r)
        } else {
            self.r - self.w
        };
    }

    pub fn size(&self) -> usize {
        return self.data.len() - self.available_size();
    }

    pub fn append_data(&mut self, data: &[u8]) -> usize {
        if self.full {
            return 0;
        }

        if self.r > self.w {
            let cap = self.r - self.w;
            let to_copy = core::cmp::min(cap, data.len());
            self.data.as_mut_slice()[self.w..self.w + to_copy].copy_from_slice(&data[..to_copy]);
            self.w += to_copy;
            self.full = self.r == self.w;
            to_copy
        } else {
            let right = self.data.len() - self.w;
            let to_copy = core::cmp::min(right, data.len());
            self.data.as_mut_slice()[self.w..self.w + to_copy].copy_from_slice(&data[..to_copy]);
            self.w = (self.w + to_copy) % self.data.len();
            self.full = self.r == self.w;
            let written = if to_copy < data.len() {
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

    pub fn read_data_transient(&self, buf: &mut [u8]) -> usize {
        if (self.r == self.w && !self.full) || buf.is_empty() {
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
                self.read_data_transient(&mut buf[to_read..])
            } else {
                0
            };
            read + to_read
        }
    }
}
