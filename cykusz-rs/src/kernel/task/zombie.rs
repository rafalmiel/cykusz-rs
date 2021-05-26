use alloc::sync::Arc;

use intrusive_collections::LinkedList;

use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::task::{SchedTaskAdapter, Task, TaskState};
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(Default)]
pub struct Zombies {
    list: Spin<LinkedList<SchedTaskAdapter>>,
    wq: WaitQueue,
}

impl Zombies {
    pub fn add_zombie(&self, zombie: Arc<Task>) {
        assert_eq!(zombie.sched.is_linked(), false);
        assert_eq!(zombie.state(), TaskState::Unused);

        let mut list = self.list.lock();

        logln_disabled!("add zombie pid {}", zombie.pid());

        list.push_back(zombie);

        self.wq.notify_one();
    }

    fn wait_on(&self, cond: impl Fn(&Task) -> bool) -> SignalResult<(usize, isize)> {
        let mut res = (0, 0);
        self.wq.wait_lock_for(&self.list, |l| {
            let mut cur = l.front_mut();

            while let Some(t) = cur.get() {
                if cond(t) {
                    res = (t.tid(), t.exit_status());

                    cur.remove();

                    return true;
                } else {
                    cur.move_next();
                }
            }

            false
        })?;

        Ok(res)
    }

    pub fn wait_thread(&self, pid: usize, tid: usize) -> SignalResult<usize> {
        self.wait_on(|t| pid == t.pid() && (t.tid() == tid || tid == 0))
            .map(|(id, _)| id)
    }

    pub fn wait_pid(&self, pid: usize, status: &mut u32) -> SignalResult<usize> {
        logln!("wait on {}", pid);
        let ret = self.wait_on(|t| t.is_process_leader() && (t.pid() == pid || pid == 0));
        logln!("wait {}0x4d3a80 finished", pid);

        return match ret {
            Ok((tid, st)) => {
                *status = 0x200; //WIFEXITED
                *status |= st as u32 & 0xff;

                Ok(tid)
            }
            Err(e) => Err(e),
        };
    }
}
