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

        list.push_back(zombie);

        drop(list);

        self.wq.notify_one();
    }

    fn wait_on(&self, cond: impl Fn(&Task) -> bool) -> SignalResult<usize> {
        let mut res = 0;
        self.wq.wait_lock_for(&self.list, |l| {
            let mut cur = l.front_mut();

            while let Some(t) = cur.get() {
                if cond(t) {
                    res = t.tid();

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
    }

    pub fn wait_pid(&self, pid: usize) -> SignalResult<usize> {
        self.wait_on(|t| t.is_process_leader() && (t.pid() == pid || pid == 0))
    }
}
