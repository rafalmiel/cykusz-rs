use core::ops::{Deref, DerefMut};

use crate::kernel::sched::current_task;
use crate::kernel::sync::spin_lock::{Spin, SpinGuard};
use crate::kernel::sync::{LockApi, LockGuard};
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct Mutex<T: ?Sized> {
    wait_queue: WaitQueue,
    mutex: Spin<T>,
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    g: Option<SpinGuard<'a, T>>,
    m: &'a Mutex<T>,
}

impl<'a, T: ?Sized + 'a> LockGuard for MutexGuard<'a, T> {}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Mutex::new(T::default())
    }
}

impl<'a, T: ?Sized + 'a> LockApi<'a, T> for Mutex<T> {
    type Guard = MutexGuard<'a, T>;

    fn lock(&self) -> MutexGuard<T> {
        let task = current_task();

        self.wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_lock() {
                self.wait_queue.remove_task(task);
                return MutexGuard {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    fn lock_irq(&self) -> MutexGuard<T> {
        let task = current_task();

        self.wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_lock_irq() {
                self.wait_queue.remove_task(task);
                return MutexGuard {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    fn try_lock(&self) -> Option<MutexGuard<T>> {
        loop {
            return if let Some(g) = self.mutex.try_lock() {
                Some(MutexGuard {
                    g: Some(g),
                    m: &self,
                })
            } else {
                None
            };
        }
    }

    fn try_lock_irq(&'a self) -> Option<Self::Guard> {
        loop {
            return if let Some(g) = self.mutex.try_lock_irq() {
                Some(MutexGuard {
                    g: Some(g),
                    m: &self,
                })
            } else {
                None
            };
        }
    }
}

impl<T> Mutex<T> {
    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            wait_queue: WaitQueue::new(),
            mutex: Spin::new_no_notify(user_data),
        }
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.g.as_ref().unwrap().deref()
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.g.as_mut().unwrap().deref_mut()
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        self.m.wait_queue.notify_one();
    }
}
