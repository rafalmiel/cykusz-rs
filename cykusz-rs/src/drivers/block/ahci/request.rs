use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;

use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::arch::raw::mm::PhysAddr;
use crate::kernel::mm::Frame;
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct DmaBuf {
    pub buf: PhysAddr,
    pub order: usize,
}

pub struct ReadRequest {
    sector: usize,
    count: usize,
    buf_vec: Vec<DmaBuf>,
    incomplete: AtomicUsize,
    wq: WaitQueue,
}

impl ReadRequest {
    pub fn new(sector: usize, count: usize) -> ReadRequest {
        let mut size = count * 512;

        let mut dma = Vec::<DmaBuf>::new();

        while size > 0 {
            let order = if size > 0x1000 { 1 } else { 0 };

            dma.push(DmaBuf {
                buf: allocate_order(order).unwrap().address(),
                order,
            });

            size -= core::cmp::min(size, 0x2000);
        }

        ReadRequest {
            sector,
            count,
            buf_vec: dma,
            incomplete: AtomicUsize::new(0),
            wq: WaitQueue::new(),
        }
    }

    pub fn dma_vec_from(&self, off: usize) -> &[DmaBuf] {
        &self.buf_vec[off / 16..]
    }

    pub fn copy_into(&self, dest: &mut [u8]) {
        let mut off = 0;
        let mut rem = self.count * 512;

        for buf in self.buf_vec.iter() {
            let cnt = core::cmp::min(rem, 0x2000);

            dest[off..off + cnt].copy_from_slice(unsafe {
                core::slice::from_raw_parts(buf.buf.to_mapped().0 as *const u8, cnt)
            });

            rem -= cnt;
            off += cnt;
        }
    }

    pub fn inc_incomplete(&self) -> usize {
        self.incomplete
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst)
            + 1
    }

    pub fn dec_incomplete(&self) -> usize {
        self.incomplete
            .fetch_sub(1, core::sync::atomic::Ordering::SeqCst)
            - 1
    }

    pub fn is_complete(&self) -> bool {
        self.incomplete.load(core::sync::atomic::Ordering::SeqCst) == 0
    }

    pub fn wait_queue(&self) -> &WaitQueue {
        &self.wq
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn sector(&self) -> usize {
        self.sector
    }
}

impl Drop for ReadRequest {
    fn drop(&mut self) {
        for buf in self.buf_vec.iter() {
            deallocate_order(&Frame::new(buf.buf), buf.order);
        }
    }
}
