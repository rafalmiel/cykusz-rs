use alloc::sync::Arc;

use crate::drivers::block::ahci::reg::{HbaPort, HbaPortISReg};
use crate::drivers::block::ata::request::DmaRequest;
use crate::kernel::block::BlockDev;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::types::CeilDiv;
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};
use bit_field::BitField;

mod hba;

struct Cmd {
    req: Arc<DmaRequest>,
}

impl Cmd {
    pub fn request(&self) -> &Arc<DmaRequest> {
        &self.req
    }
}

struct PortData {
    cmds: [Option<Cmd>; 32],
    port: VirtAddr,
    free_cmds: usize,
}

pub struct Port {
    data: Spin<PortData>,
    cmd_wq: WaitQueue,
}

impl PortData {
    fn hba_port(&mut self) -> &mut HbaPort {
        unsafe { self.port.read_mut::<HbaPort>() }
    }

    fn handle_interrupt(&mut self) {
        let port = self.hba_port();

        let is = port.is();
        port.set_is(is);

        let ci = port.ci() | port.sact();

        if !is.is_set(HbaPortISReg::DHRS) {
            port.set_is(is);
            return;
        }

        for (i, cmd) in self.cmds.iter_mut().enumerate() {
            if !ci.get_bit(i) {
                if let Some(request) = if let Some(cmd_inner) = cmd {
                    Some(cmd_inner.request().clone())
                } else {
                    None
                } {
                    let fin = request.dec_incomplete() == 0;

                    *cmd = None;

                    self.free_cmds += 1;

                    if fin {
                        request.wait_queue().notify_one();
                    }
                }
            }
        }
        //let port = self.hba_port();
        //port.set_is(is);
    }

    fn find_cmd_slot(&mut self) -> Option<usize> {
        self.cmds
            .iter()
            .enumerate()
            .find_map(|(i, e)| if e.is_none() { Some(i) } else { None })
    }

    fn run_request(&mut self, request: Arc<DmaRequest>, mut off: usize) -> usize {
        let mut rem = request.count() - off;

        while rem > 0 {
            if let Some(slot) = self.find_cmd_slot() {
                self.cmds[slot] = Some(Cmd {
                    req: request.clone(),
                });

                self.free_cmds -= 1;

                request.inc_incomplete();

                let port = self.hba_port();

                let cnt = core::cmp::min(rem, 128);

                port.run_command(
                    request.ata_command(),
                    request.sector() + off,
                    cnt,
                    request.dma_vec_from(off),
                    slot,
                );

                rem -= cnt;
                off += cnt;
            } else {
                return off;
            }
        }

        off
    }
}

impl Port {
    pub fn new(addr: VirtAddr) -> Port {
        const EMPTY: Option<Cmd> = None;
        Port {
            data: Spin::new(PortData {
                cmds: [EMPTY; 32],
                port: addr,
                free_cmds: 32,
            }),
            cmd_wq: WaitQueue::new(),
        }
    }

    pub fn handle_interrupt(&self) {
        let mut data = self.data.lock_irq();

        data.handle_interrupt();

        if data.free_cmds > 0 {
            self.cmd_wq.notify_all();
        }
    }

    fn run_request(&self, request: Arc<DmaRequest>) -> Option<usize> {
        let mut off = 0;
        // post request and wait for completion.....
        while off < request.count() {
            let mut data = self
                .cmd_wq
                .wait_lock_for(
                    WaitQueueFlags::IRQ_DISABLE | WaitQueueFlags::NON_INTERRUPTIBLE,
                    &self.data,
                    |d| d.free_cmds > 0,
                )
                .unwrap()
                .unwrap();

            off = data.run_request(request.clone(), off);
        }

        request
            .wait_queue()
            .wait_for(
                WaitQueueFlags::IRQ_DISABLE | WaitQueueFlags::NON_INTERRUPTIBLE,
                || request.is_complete(),
            )
            .unwrap();

        Some(request.count() * 512)
    }
}

impl BlockDev for Port {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        let count = dest.len().ceil_div(512);

        let request = Arc::new(DmaRequest::new(sector, count));

        let start = crate::kernel::timer::current_ns();

        let res = self.run_request(request.clone());
        dbgln!(ahci, "run_request end {}: {} us", crate::cpu_id(), (crate::kernel::timer::current_ns() - start) / 1000);

        if let Some(r) = &res {
            if *r / 512 != count {
                println!("warn: read only {} bytes?", *r);
            }
            request.copy_into(dest);
        }

        res
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        let request = Arc::new(DmaRequest::from_bytes(sector, buf));

        self.run_request(request)
    }
}
