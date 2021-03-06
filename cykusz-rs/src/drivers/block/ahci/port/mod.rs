use alloc::sync::Arc;

use bit_field::BitField;

use crate::drivers::block::ahci::reg::HbaPort;
use crate::drivers::block::ata::request::DmaRequest;
use crate::kernel::block::BlockDev;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sync::Spin;
use crate::kernel::utils::types::CeilDiv;
use crate::kernel::utils::wait_queue::WaitQueue;

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
        port.set_is(port.is());

        let ci = port.ci();

        for (i, cmd) in self.cmds.iter_mut().enumerate() {
            if !ci.get_bit(i) {
                if let Some(cmd_inner) = cmd {
                    let fin = cmd_inner.request().dec_incomplete() == 0;

                    if fin {
                        cmd_inner.request().wait_queue().notify_one();
                    }

                    *cmd = None;

                    self.free_cmds += 1;
                }
            }
        }
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
            let slot = {
                if let Some(slot) = self.find_cmd_slot() {
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

                    slot
                } else {
                    return off;
                }
            };

            self.cmds[slot] = Some(Cmd {
                req: request.clone(),
            });

            self.free_cmds -= 1;

            request.inc_incomplete();
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
        //let is_int = crate::kernel::int::is_enabled();

        //if !is_int {
        //    crate::kernel::int::enable();
        //}

        let mut off = 0;
        // post request and wait for completion.....
        while off < request.count() {
            let data = self
                .cmd_wq
                .wait_lock_irq_for(&self.data, |d| d.free_cmds > 0);

            match data {
                Ok(mut l) => {
                    off = l.run_request(request.clone(), off);
                }
                Err(_e) => {
                    return Some(off * 512);
                }
            }
        }

        while let Err(_e) = request.wait_queue().wait_for_irq(|| request.is_complete()) {
            // TODO: Make some waits uninterruptible
            //println!("[ AHCI ] IO interrupted, retrying");
        }

        //if !is_int {
        //    crate::kernel::int::disable();
        //}

        Some(request.count() * 512)
    }
}

impl BlockDev for Port {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        let count = dest.len().ceil_div(512);

        let request = Arc::new(DmaRequest::new(sector, count));

        let res = self.run_request(request.clone());

        if res.is_some() {
            request.copy_into(dest);
        }

        res
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        let request = Arc::new(DmaRequest::from_bytes(sector, buf));

        self.run_request(request.clone())
    }
}
