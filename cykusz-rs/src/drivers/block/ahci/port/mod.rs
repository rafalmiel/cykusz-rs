use alloc::sync::Arc;

use bit_field::BitField;

use crate::drivers::block::ahci::reg::HbaPort;
use crate::drivers::block::ahci::request::ReadRequest;
use crate::kernel::device::block::BlockDev;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;

mod hba;

struct Cmd {
    req: Arc<ReadRequest>,
}

impl Cmd {
    pub fn request(&self) -> &Arc<ReadRequest> {
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
        let ci = port.ci();

        port.set_is(port.is());

        for (i, cmd) in self.cmds.iter_mut().enumerate() {
            if !ci.get_bit(i) {
                if if let Some(cmd) = cmd {
                    let fin = cmd.request().dec_incomplete() == 0;

                    if fin {
                        cmd.request().wait_queue().notify_one();
                    }

                    true
                } else {
                    false
                } {
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

    fn read(&mut self, request: Arc<ReadRequest>, mut off: usize) -> usize {
        let mut rem = request.count() - off;

        while rem > 0 {
            let slot = {
                if let Some(slot) = self.find_cmd_slot() {
                    let port = self.hba_port();

                    let cnt = core::cmp::min(rem, 128);

                    port.read(request.sector() + off, cnt, request.dma_vec_from(off), slot);

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
        Port {
            data: Spin::new(PortData {
                cmds: [None; 32],
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
}

impl BlockDev for Port {
    fn read(&self, sector: usize, count: usize, dest: &mut [u8]) -> Option<usize> {
        if dest.len() < count * 512 {
            return None;
        }

        let req = Arc::new(ReadRequest::new(sector, count));

        let mut off = 0;
        // post request and wait for completion.....
        while off < count {
            let mut data = self
                .cmd_wq
                .wait_lock_irq_for(&self.data, |d| d.free_cmds > 0);

            off = data.read(req.clone(), off);
        }

        req.wait_queue().wait_for(|| req.is_complete());

        req.copy_into(dest);

        Some(count * 512)
    }

    fn write(&self, _sector: usize, _buf: &[u8]) -> Option<usize> {
        unimplemented!()
    }
}
