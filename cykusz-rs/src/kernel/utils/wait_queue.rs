use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use crate::kernel::sync::Mutex;
use crate::kernel::task::{Task, TaskState};

pub struct WaitQueue {
    tasks: Mutex<Vec<Weak<Task>>>,
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn add_task(&self, task: Arc<Task>) {
        debug_assert_eq!(task.locks(), 0, "AwaitintIo while holding a lock");

        {
            let mut l = self.tasks.lock_irq();
            l.push(Arc::<Task>::downgrade(&task));
            task.set_state(TaskState::AwaitingIo);
        }

        crate::kernel::sched::reschedule();
    }

    pub fn notify_one(&self) -> bool {
        let mut tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        for i in (0..len).rev() {
            if let Some(t) = tasks[i].upgrade() {
                t.set_state(TaskState::Runnable);
                tasks.remove(i);
                return true;
            }

            tasks.remove(i);
        }

        false
    }
}
