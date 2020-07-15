use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::task::{Task, TaskState};

pub struct WaitQueue {
    tasks: Spin<Vec<Weak<Task>>>,
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            tasks: Spin::new(Vec::new()),
        }
    }

    pub fn wait(&self) {
        let task = current_task();

        self.add_task(task);
    }

    fn add_task(&self, task: Arc<Task>) {
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

    pub fn notify_all(&self) -> bool {
        let mut tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in (0..len).rev() {
            if let Some(t) = tasks[i].upgrade() {
                t.set_state(TaskState::Runnable);
                res = true;
            }
        }

        tasks.clear();

        res
    }
}
