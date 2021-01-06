use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;

use crate::drivers::block::ahci::reg::AtaCommand;
use crate::kernel::mm::Frame;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::{allocate_order, deallocate_order};
use crate::kernel::utils::types::CeilDiv;
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct DmaBuf {
    pub buf: PhysAddr,
    pub order: usize,
}

#[derive(PartialOrd, PartialEq)]
pub enum DmaCommand {
    Read,
    Write,
}

pub struct DmaRequest {
    sector: usize,
    count: usize,
    buf_vec: Vec<DmaBuf>,
    incomplete: AtomicUsize,
    wq: WaitQueue,
    command: DmaCommand,
}

impl DmaRequest {
    pub fn new(sector: usize, count: usize) -> DmaRequest {
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

        DmaRequest {
            sector,
            count,
            buf_vec: dma,
            incomplete: AtomicUsize::new(0),
            wq: WaitQueue::new(),
            command: DmaCommand::Read,
        }
    }

    pub fn from_bytes(sector: usize, buf: &[u8]) -> DmaRequest {
        let count = buf.len().ceil_div(512);

        let mut req = Self::new(sector, count);

        req.command = DmaCommand::Write;

        req.copy_from(buf);

        req
    }

    pub fn dma_vec_from(&self, off: usize) -> &[DmaBuf] {
        &self.buf_vec[off / 16..]
    }

    pub fn copy_into(&self, dest: &mut [u8]) {
        let mut off = 0;
        let mut rem = dest.len();

        for buf in self.buf_vec.iter() {
            let cnt = core::cmp::min(rem, 0x2000);

            dest[off..off + cnt].copy_from_slice(unsafe { buf.buf.to_mapped().as_bytes(cnt) });

            rem -= cnt;
            off += cnt;
        }
    }

    fn copy_from(&mut self, src: &[u8]) {
        use core::cmp::min;

        let mut off = 0;

        for b in self.buf_vec.iter_mut() {
            let size = min(
                match b.order {
                    0 => 0x1000,
                    1 => 2 * 0x1000,
                    _ => unreachable!(),
                },
                src.len() - off,
            );

            unsafe {
                b.buf
                    .to_mapped()
                    .as_bytes_mut(size)
                    .copy_from_slice(&src[off..off + size]);
            }

            off += size;
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

    pub fn ata_command(&self) -> AtaCommand {
        match self.command {
            DmaCommand::Read => AtaCommand::AtaCommandReadDmaExt,
            DmaCommand::Write => AtaCommand::AtaCommandWriteDmaExt,
        }
    }
}

impl Drop for DmaRequest {
    fn drop(&mut self) {
        for buf in self.buf_vec.iter() {
            deallocate_order(&Frame::new(buf.buf), buf.order);
        }
    }
}
