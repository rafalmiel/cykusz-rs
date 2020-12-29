use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::arch::raw::mm::PhysAddr;
use crate::kernel::mm::Frame;
use crate::kernel::utils::wait_queue::WaitQueue;
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;

pub struct DmaBuf {
    pub buf: PhysAddr,
    pub order: usize,
}

pub struct ReadRequest {
    pub sector: usize,
    pub count: usize,
    pub buf_vec: Vec<DmaBuf>,
    pub incomplete: AtomicUsize,
    pub wq: WaitQueue,
}

impl Drop for ReadRequest {
    fn drop(&mut self) {
        for buf in self.buf_vec.iter() {
            deallocate_order(&Frame::new(buf.buf), buf.order);
        }
    }
}

pub fn make_request(sector: usize, count: usize) -> ReadRequest {
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
