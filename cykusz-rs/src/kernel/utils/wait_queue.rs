use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::sched::current_task;
use crate::kernel::sync::{Spin, SpinGuard};
use crate::kernel::task::{Task, TaskState};

pub struct WaitQueue {
    tasks: Spin<Vec<Arc<Task>>>,
}

impl Default for WaitQueue {
    fn default() -> WaitQueue {
        WaitQueue {
            tasks: Spin::new(Vec::new()),
        }
    }
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            tasks: Spin::new(Vec::new()),
        }
    }

    pub fn wait_lock<T>(lock: SpinGuard<T>) {
        let task = current_task();

        core::mem::drop(lock);

        task.await_io();
    }

    pub fn task_wait() {
        let task = current_task();

        task.await_io();
    }

    pub fn wait(&self) {
        let task = current_task();

        self.add_task(task.clone());

        task.await_io();

        self.remove_task(task);
    }

    pub fn wait_lock_irq_for<'a, T, F: Fn(&SpinGuard<T>) -> bool>(
        &self,
        mtx: &'a Spin<T>,
        cond: F,
    ) -> SpinGuard<'a, T> {
        let mut lock = mtx.lock_irq();

        if cond(&lock) {
            return lock;
        }

        let task = current_task();

        self.add_task(task.clone());

        while !cond(&lock) {
            core::mem::drop(lock);

            task.await_io();

            lock = mtx.lock_irq();
        }

        self.remove_task(task);

        lock
    }

    pub fn wait_lock_for<'a, T, F: Fn(&SpinGuard<T>) -> bool>(
        &self,
        mtx: &'a Spin<T>,
        cond: F,
    ) -> SpinGuard<'a, T> {
        let mut lock = mtx.lock();

        if cond(&lock) {
            return lock;
        }

        let task = current_task();

        self.add_task(task.clone());

        while !cond(&lock) {
            core::mem::drop(lock);

            task.await_io();

            lock = mtx.lock();
        }

        self.remove_task(task);

        lock
    }

    pub fn wait_for<F: Fn() -> bool>(&self, cond: F) {
        let task = current_task();

        self.add_task(task.clone());

        while !cond() {
            task.await_io();
        }

        self.remove_task(task);
    }

    pub fn add_task(&self, task: Arc<Task>) {
        let mut tasks = self.tasks.lock_irq();

        tasks.push(task);
    }

    pub fn remove_task(&self, task: Arc<Task>) {
        let mut tasks = self.tasks.lock_irq();

        if let Some(idx) = tasks.iter().enumerate().find_map(|e| {
            let t = e.1;

            if t.id() == task.id() {
                return Some(e.0);
            }

            None
        }) {
            tasks.remove(idx);
        }
    }

    pub fn notify_one(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        for i in (0..len).rev() {
            let t = tasks[i].clone();

            if t.state() == TaskState::AwaitingIo {
                t.set_state(TaskState::Runnable);
            } else {
                t.set_has_pending_io(true);
            }

            return true;
        }

        false
    }
    pub fn notify_one_debug(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        for i in (0..len).rev() {
            let t = tasks[i].clone();

            if t.state() == TaskState::AwaitingIo {
                t.set_state(TaskState::Runnable);
            } else {
                t.set_has_pending_io(true);
            }

            return true;
        }

        false
    }

    pub fn notify_all(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in (0..len).rev() {
            let t = tasks[i].clone();

            if t.state() == TaskState::AwaitingIo {
                t.set_state(TaskState::Runnable);
            } else {
                t.set_has_pending_io(true);
            }
            res = true;
        }

        res
    }
}
