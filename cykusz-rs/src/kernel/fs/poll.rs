use alloc::vec::Vec;
use intrusive_collections::UnsafeRef;
use crate::kernel::sched::current_task;
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct PollTable {
    queues: Vec<UnsafeRef<WaitQueue>>,
}

impl PollTable {
    pub fn new(capacity: usize) -> PollTable {
        PollTable {
            queues: Vec::with_capacity(capacity as usize),
        }
    }

    pub fn listen(&mut self, queue: &WaitQueue) {
        queue.add_task(current_task());
        self.queues
            .push(unsafe { UnsafeRef::from_raw(queue as *const _) });
    }
}

impl Drop for PollTable {
    fn drop(&mut self) {
        let task = current_task();
        for q in &self.queues {
            q.remove_task(task.clone());
        }
    }
}
