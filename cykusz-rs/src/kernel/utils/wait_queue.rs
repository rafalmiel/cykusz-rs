use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::sched::current_task;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::{Spin, SpinGuard};
use crate::kernel::task::Task;

pub struct WaitQueue {
    tasks: Spin<Vec<Arc<Task>>>,
}

pub struct WaitQueueGuard<'a> {
    wq: &'a WaitQueue,
    task: &'a Arc<Task>,
}

impl<'a> WaitQueueGuard<'a> {
    pub fn new(wq: &'a WaitQueue, task: &'a Arc<Task>) -> WaitQueueGuard<'a> {
        wq.add_task(task.clone());

        WaitQueueGuard::<'a> { wq, task }
    }
}

impl<'a> Drop for WaitQueueGuard<'a> {
    fn drop(&mut self) {
        self.wq.remove_task(self.task.clone());
    }
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

    pub fn is_empty(&self) -> bool {
        self.tasks.lock().is_empty()
    }

    pub fn wait_lock<T>(lock: SpinGuard<T>) -> SignalResult<()> {
        let task = current_task();

        core::mem::drop(lock);

        task.await_io()
    }

    pub fn task_wait() -> SignalResult<()> {
        let task = current_task();

        task.await_io()
    }

    pub fn wait(&self) -> SignalResult<()> {
        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        task.await_io()
    }

    pub fn wait_lock_irq_for<'a, T, F: FnMut(&mut SpinGuard<T>) -> bool>(
        &self,
        mtx: &'a Spin<T>,
        mut cond: F,
    ) -> SignalResult<SpinGuard<'a, T>> {
        let mut lock = mtx.lock_irq();

        if cond(&mut lock) {
            return Ok(lock);
        }

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond(&mut lock) {
            core::mem::drop(lock);

            task.await_io()?;

            lock = mtx.lock_irq();
        }

        Ok(lock)
    }

    pub fn wait_lock_for<'a, T, F: FnMut(&mut SpinGuard<T>) -> bool>(
        &self,
        mtx: &'a Spin<T>,
        mut cond: F,
    ) -> SignalResult<SpinGuard<'a, T>> {
        let mut lock = mtx.lock();

        if cond(&mut lock) {
            return Ok(lock);
        }

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond(&mut lock) {
            core::mem::drop(lock);

            task.await_io()?;

            lock = mtx.lock();
        }

        Ok(lock)
    }

    pub fn wait_for<F: FnMut() -> bool>(&self, mut cond: F) -> SignalResult<()> {
        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond() {
            task.await_io()?;
        }

        Ok(())
    }

    pub fn add_task(&self, task: Arc<Task>) {
        let mut tasks = self.tasks.lock_irq();

        tasks.push(task);
    }

    pub fn remove_task(&self, task: Arc<Task>) {
        let mut tasks = self.tasks.lock_irq();

        if let Some(idx) = tasks.iter().enumerate().find_map(|e| {
            let t = e.1;

            if t.tid() == task.tid() {
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

        let t = tasks.first().unwrap();

        t.wake_up();

        return true;
    }

    pub fn notify_group(&self, gid: usize) -> bool {
        let tasks = self.tasks.lock_irq();

        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in 0..len {
            let t = tasks[i].clone();

            if t.gid() == gid {
                t.wake_up();

                res = true;
            }
        }

        return res;
    }

    pub fn notify_one_debug(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let t = tasks.first().unwrap();

        println!("wake up {}", t.tid());

        t.wake_up();

        return true;
    }

    pub fn notify_all(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in 0..len {
            let t = tasks[i].clone();

            t.wake_up();

            res = true;
        }

        res
    }

    pub fn notify_all_debug(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in 0..len {
            let t = tasks[i].clone();

            println!("wake up {}", t.tid());
            t.wake_up();

            res = true;
        }

        res
    }
}
