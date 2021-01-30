use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::kernel::sync::SpinGuard;
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

        let task = Arc::new(Task::this());

        this.tasks.push(task.clone());

        CURRENT_TASK_ID.store(task.id(), Ordering::SeqCst);

        this
    }
}

impl CpuQueue {
    pub fn register_main_task(&self) {
        crate::kernel::sched::register_task(self.tasks[0].clone());
    }

    fn switch(&self, to: &Task, lock: SpinGuard<()>) {
        drop(lock);

        self.finalize();
        CURRENT_TASK_ID.store(to.id(), Ordering::SeqCst);

        unsafe {
            switch!(&self.sched_task, &to);
        }
    }

    fn switch_to_sched(&self, from: &Task, lock: SpinGuard<()>) {
        drop(lock);

        unsafe {
            switch!(&from, &self.sched_task);
        }
    }

    fn _activate_sched(&self, lock: SpinGuard<()>) {
        drop(lock);

        unsafe { activate_task!(&self.sched_task) }
    }

    fn finalize(&self) {
        crate::kernel::int::finish();
        crate::kernel::timer::reset_counter();
    }

    pub fn current_task(&self, _lock: SpinGuard<()>) -> Arc<Task> {
        self.tasks[self.current].clone()
    }

    pub unsafe fn schedule_next(&mut self, sched_lock: SpinGuard<()>) {
        let current = &self.tasks[self.current];
        if current.to_delete() {
            self.remove_task(self.current);

            if self.current != 0 {
                self.current -= 1;
            } else {
                panic!("Removing main task?")
            }

            self.schedule_next(sched_lock);
            return;
        } else if current.locks() > 0 && current.state() == TaskState::Running {
            current.set_to_reschedule(true);

            self.switch(current, sched_lock);

            return;
        } else if current.has_pending_io() {
            current.set_has_pending_io(false);

            current.set_state(TaskState::Running);
            self.switch(current, sched_lock);

            return;
        }

        if self.tasks.len() == 1 {
            self.switch(&self.tasks[0], sched_lock);
            return;
        }

        let len = self.tasks.len();

        let mut c = (self.current % (len - 1)) + 1;
        let mut loops = 0;

        let current_ns = crate::kernel::timer::current_ns() as usize;

        let found = loop {
            let task = &self.tasks[c];

            let state = task.state();

            if state == TaskState::AwaitingIo {
                let t = task.sleep_until.load(Ordering::SeqCst);
                if t != 0 && current_ns > t {
                    task.sleep_until.store(0, Ordering::SeqCst);
                    task.set_state(TaskState::Runnable);
                    break Some(c);
                }
            }

            if state == TaskState::Runnable || (state == TaskState::Running && c == self.current) {
                break Some(c);
            } else if loops == len - 1 {
                break Some(0);
            }

            c = (c % (len - 1)) + 1;
            loops += 1;
        }
        .expect("SCHEDULER BUG");

        if current.state() == TaskState::Running {
            current.set_state(TaskState::Runnable);
        }

        self.tasks[found].set_state(TaskState::Running);

        self.previous = self.current;
        self.current = found;

        QUEUE_LEN.store(self.tasks.len(), Ordering::SeqCst);

        self.switch(&self.tasks[found], sched_lock);
    }

    pub fn reschedule(&mut self, sched_lock: SpinGuard<()>) -> bool {
        self.switch_to_sched(&self.tasks[self.current], sched_lock);

        return self.current != self.previous;
    }

    pub fn activate_sched(&mut self, sched_lock: SpinGuard<()>) {
        self._activate_sched(sched_lock);
    }

    pub fn enter_critical_section(&mut self) {
        self.tasks[self.current].locks_inc();
    }

    pub fn leave_critical_section(&mut self, mutex: SpinGuard<()>) {
        let t = &self.tasks[self.current];

        t.locks_dec();

        if t.locks() == 0 && t.to_reschedule() {
            t.set_to_reschedule(false);

            self.reschedule(mutex);
        }
    }

    pub fn current_task_finished(&mut self, lock: SpinGuard<()>) -> ! {
        let task = &self.tasks[self.current];

        task.set_to_delete(true);
        self.switch_to_sched(task, lock);

        unreachable!()
    }

    pub fn add_task(&mut self, task: Arc<Task>) {
        let _lock_protect = RecursiveLockProtection::new();

        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, idx: usize) {
        let _lock_protect = RecursiveLockProtection::new();

        if Arc::strong_count(&self.tasks[idx]) != 1 {
            println!(
                "Deallocating task with {} alive references, id {}",
                Arc::strong_count(&self.tasks[idx]),
                self.tasks[idx].id()
            );
        }

        self.tasks.remove(idx);
        self.tasks.shrink_to_fit();
    }
}
