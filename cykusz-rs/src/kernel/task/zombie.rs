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

    pub fn wait_pid(&self, pid: usize) -> SignalResult<()> {
        self.wq.wait_lock_for(&self.list, |l| {
            let mut cur = l.front_mut();

            while let Some(t) = cur.get() {
                if t.id() == pid {
                    cur.remove();

                    return true;
                } else {
                    cur.move_next();
                }
            }

            false
        })?;

        Ok(())
    }
}
