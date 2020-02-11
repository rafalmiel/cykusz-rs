use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::kernel::sync::MutexGuard;
use crate::kernel::task::{Task, TaskState};

use super::CURRENT_TASK_ID;
use super::LOCK_PROTECTION_ENTERED;
use super::QUEUE_LEN;

struct RecursiveLockProtection {}

impl RecursiveLockProtection {
    fn new() -> RecursiveLockProtection {
        LOCK_PROTECTION_ENTERED.store(true, Ordering::SeqCst);
        RecursiveLockProtection {}
    }
}

impl Drop for RecursiveLockProtection {
    fn drop(&mut self) {
        LOCK_PROTECTION_ENTERED.store(false, Ordering::SeqCst);
    }
}

pub struct CpuQueue {
    sched_task: Task,
    tasks: Vec<Arc<Task>>,
    current: usize,
    previous: usize,
}

impl Default for CpuQueue {
    fn default() -> CpuQueue {
        let mut this = CpuQueue {
            sched_task: Task::new_sched(super::scheduler_main),
            tasks: Vec::new(),
            current: 0,
            previous: 0,
        };

        this.tasks.push(Arc::new(Task::this()));

        this
    }
}

impl CpuQueue {
    fn switch(&self, to: &Task, lock: MutexGuard<()>) {
        drop(lock);

        self.finalize();
        CURRENT_TASK_ID.store(to.id(), Ordering::SeqCst);

        unsafe {
            switch!(&self.sched_task, &to);
        }
    }

    fn switch_to_sched(&self, from: &Task, lock: MutexGuard<()>) {
        drop(lock);

        unsafe {
            switch!(&from, &self.sched_task);
        }
    }

    fn activate_sched(&self, lock: MutexGuard<()>) -> ! {
        drop(lock);

        unsafe {
            activate_task!(&self.sched_task)
        }

        unreachable!()
    }

    fn finalize(&self) {
        crate::kernel::int::finish();
        crate::kernel::timer::reset_counter();
    }

    pub fn current_task(&self, _lock: MutexGuard<()>) -> Arc<Task> {
        self.tasks[self.current].clone()
    }

    pub unsafe fn schedule_next(&mut self, sched_lock: MutexGuard<()>) {
        if self.tasks[self.current].state() == TaskState::ToDelete {
            self.remove_task(self.current);

            if self.current != 0 {
                self.current -= 1;
            }

            self.schedule_next(sched_lock);
            return;
        } else if self.tasks[self.current].locks() > 0 {
            self.tasks[self.current].mark_to_reschedule();

            self.switch(&self.tasks[self.current], sched_lock);

            return;
        }

        if self.tasks.len() == 1 {
            self.switch(&self.tasks[0], sched_lock);
            return;
        }

        let len = self.tasks.len();

        let mut c = (self.current % (len - 1)) + 1;
        let mut loops = 0;

        let found = loop {
            let state = self.tasks[c].state();
            if state == TaskState::Runnable {
                break Some(c);
            } else if c == self.current && self.tasks[self.current].state() == TaskState::Running {
                break Some(self.current);
            } else if loops == len - 1 {
                break Some(0);
            }

            c = (c % (len - 1)) + 1;
            loops += 1;
        }
        .expect("SCHEDULER BUG");

        if self.tasks[self.current].state() == TaskState::Running {
            self.tasks[self.current].set_state(TaskState::Runnable);
        }

        self.tasks[found].set_state(TaskState::Running);

        self.previous = self.current;
        self.current = found;

        QUEUE_LEN.store(self.tasks.len(), Ordering::SeqCst);

        self.switch(&self.tasks[found], sched_lock);
    }

    pub fn reschedule(&mut self, sched_lock: MutexGuard<()>) -> bool {
        self.switch_to_sched(&self.tasks[self.current], sched_lock);

        return self.current != self.previous;
    }

    pub fn enter_critical_section(&mut self) {
        self.tasks[self.current].locks_inc();
    }

    pub fn leave_critical_section(&mut self, mutex: MutexGuard<()>) {
        let t = &self.tasks[self.current];

        t.locks_dec();

        if t.locks() == 0 && t.state() == TaskState::ToReschedule {
            t.unmark_to_reschedule();

            self.reschedule(mutex);
        }
    }

    pub fn current_task_finished(&mut self, lock: MutexGuard<()>) -> ! {
        //println!("Strong count: {}", Arc::strong_count(&self.tasks[self.current]));
        self.tasks[self.current].set_state(TaskState::ToDelete);
        self.activate_sched(lock)
    }

    pub fn add_task(&mut self, task: Arc<Task>) {
        let _lock_protect = RecursiveLockProtection::new();

        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, idx: usize) {
        let _lock_protect = RecursiveLockProtection::new();

        self.tasks.remove(idx);
        self.tasks.shrink_to_fit();
    }
}
