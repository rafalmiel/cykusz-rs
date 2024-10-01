use alloc::sync::Arc;
use alloc::vec::Vec;
use syscall_defs::net::MsgFlags;
use syscall_defs::OpenFlags;

use crate::kernel::sched::{current_task, SleepFlags};
use crate::kernel::signal::{SignalError, SignalResult};
use crate::kernel::sync::{IrqGuard, LockApi, LockGuard, Spin};
use crate::kernel::task::Task;

pub struct WaitQueue {
    tasks: Spin<Vec<Arc<Task>>>,
}

pub struct WaitQueueGuard<'a> {
    wq: &'a WaitQueue,
    task: &'a Arc<Task>,
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct WaitQueueFlags: u64 {
        const IRQ_DISABLE = (1u64 << 0);
        const NON_INTERRUPTIBLE = (1u64 << 1);
        const NO_HANG = (1u64 << 2);
    }
}

impl From<WaitQueueFlags> for SleepFlags {
    fn from(value: WaitQueueFlags) -> Self {
        let mut flags = SleepFlags::empty();
        if value.contains(WaitQueueFlags::NON_INTERRUPTIBLE) {
            flags.insert(SleepFlags::NON_INTERRUPTIBLE);
        }

        flags
    }
}

impl From<OpenFlags> for WaitQueueFlags {
    fn from(value: OpenFlags) -> Self {
        if value.contains(OpenFlags::NONBLOCK) {
            WaitQueueFlags::NO_HANG
        } else {
            WaitQueueFlags::empty()
        }
    }
}

impl From<MsgFlags> for WaitQueueFlags {
    fn from(value: MsgFlags) -> Self {
        if value.contains(MsgFlags::MSG_DONTWAIT) {
            WaitQueueFlags::NO_HANG
        } else {
            WaitQueueFlags::empty()
        }
    }
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

    pub fn with_capacity(size: usize) -> WaitQueue {
        WaitQueue {
            tasks: Spin::new(Vec::with_capacity(size)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.lock_irq().is_empty()
    }

    pub fn wait_lock<G: LockGuard>(lock: G) -> SignalResult<()> {
        let task = current_task();

        core::mem::drop(lock);

        task.await_io(SleepFlags::empty())
    }

    pub fn task_wait() -> SignalResult<()> {
        let task = current_task();

        task.await_io(SleepFlags::empty())
    }

    pub fn wait(&self, flags: WaitQueueFlags) -> SignalResult<()> {
        let _irq = if flags.contains(WaitQueueFlags::IRQ_DISABLE) {
            Some(IrqGuard::new())
        } else {
            None
        };

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        Self::do_await_io(flags, &task)
    }

    pub fn wait_lock_for<'a, T, G: LockApi<'a, T>, F: FnMut(&mut G::Guard) -> bool>(
        &self,
        flags: WaitQueueFlags,
        mtx: &'a G,
        mut cond: F,
    ) -> SignalResult<Option<G::Guard>> {
        let irq = flags.contains(WaitQueueFlags::IRQ_DISABLE);
        let mut lock = if irq { mtx.lock_irq() } else { mtx.lock() };

        if cond(&mut lock) {
            return Ok(Some(lock));
        } else if flags.contains(WaitQueueFlags::NO_HANG) {
            return Ok(None);
        }

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond(&mut lock) {
            drop(lock);

            Self::do_await_io(flags, &task)?;

            lock = if irq { mtx.lock_irq() } else { mtx.lock() };
        }

        Ok(Some(lock))
    }

    pub fn wait_lock_for_debug<'a, T, G: LockApi<'a, T>, F: FnMut(&mut G::Guard) -> bool>(
        &self,
        flags: WaitQueueFlags,
        mtx: &'a G,
        debug: usize,
        mut cond: F,
    ) -> SignalResult<Option<G::Guard>> {
        let irq = flags.contains(WaitQueueFlags::IRQ_DISABLE);
        let mut lock = if irq {
            mtx.lock_irq_debug(debug)
        } else {
            mtx.lock_debug(debug)
        };

        if cond(&mut lock) {
            return Ok(Some(lock));
        } else if flags.contains(WaitQueueFlags::NO_HANG) {
            return Ok(None);
        }

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond(&mut lock) {
            drop(lock);

            Self::do_await_io(flags, &task)?;

            lock = if irq {
                mtx.lock_irq_debug(debug)
            } else {
                mtx.lock_debug(debug)
            };
        }

        Ok(Some(lock))
    }

    pub fn wait_for<F: FnMut() -> bool>(
        &self,
        flags: WaitQueueFlags,
        mut cond: F,
    ) -> SignalResult<Option<()>> {
        if !cond() && flags.contains(WaitQueueFlags::NO_HANG) {
            return Ok(None);
        }

        let _irq = if flags.contains(WaitQueueFlags::IRQ_DISABLE) {
            Some(IrqGuard::new())
        } else {
            None
        };

        let task = current_task();

        let _guard = WaitQueueGuard::new(self, &task);

        while !cond() {
            Self::do_await_io(flags, &task)?;
        }

        Ok(Some(()))
    }

    fn do_await_io(flags: WaitQueueFlags, task: &Arc<Task>) -> SignalResult<()> {
        let res = task.await_io(flags.into());

        return match res {
            Ok(()) => Ok(()),
            res @ Err(SignalError::Interrupted) => {
                if flags.contains(WaitQueueFlags::NON_INTERRUPTIBLE) {
                    Ok(())
                } else {
                    res
                }
            }
        };
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

        true
    }

    pub fn notify_group(&self, gid: usize) -> bool {
        let tasks = self.tasks.lock_irq();

        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in 0..len {
            let t = &tasks[i];

            if t.gid() == gid {
                t.wake_up();

                res = true;
            }
        }

        res
    }

    pub fn notify_all(&self) -> bool {
        let tasks = self.tasks.lock_irq();
        let len = tasks.len();

        if len == 0 {
            return false;
        }

        let mut res = false;

        for i in 0..len {
            let t = &tasks[i];

            t.wake_up();

            res = true;
        }

        res
    }

    pub fn signal_all(&self, sig: usize) {
        let tasks = self.tasks.lock_irq();

        for t in tasks.iter() {
            if !t.signal(sig) {
                t.wake_up();
            }
        }
    }
}
