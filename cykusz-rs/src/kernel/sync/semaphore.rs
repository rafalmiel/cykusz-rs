use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct Semaphore {
    max_value: isize,
    internals: Spin<Inner>,
    wait_queue: WaitQueue,
}

struct Inner {
    value: isize,
}

impl Semaphore {
    pub const fn new(init_val: isize, max_val: isize) -> Semaphore {
        Semaphore {
            max_value: max_val,
            internals: Spin::new(Inner { value: init_val }),
            wait_queue: WaitQueue::new(),
        }
    }

    pub fn acquire(&self) {
        let mut lh = self.internals.lock();
        if lh.value < 1 {
            core::mem::drop(lh);
            self.wait_queue.add_task(current_task());
        } else {
            lh.value -= 1;
        }
    }

    pub fn release(&self) {
        let mut lh = self.internals.lock();

        if !self.wait_queue.notify_one() {
            if lh.value < self.max_value {
                lh.value += 1;
            }
        }
    }
}
