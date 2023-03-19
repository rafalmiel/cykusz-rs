use core::ops::{Deref, DerefMut};

use crate::kernel::sched::current_task;
use crate::kernel::sync::spin_rw_lock::RwSpinUpgradeableGuard;
use crate::kernel::sync::{IrqGuard, RwSpin, RwSpinReadGuard, RwSpinWriteGuard};
use crate::kernel::utils::wait_queue::WaitQueue;

pub struct RwMutex<T: ?Sized> {
    reader_wait_queue: WaitQueue,
    writer_wait_queue: WaitQueue,
    mutex: RwSpin<T>,
}

pub struct RwMutexReadGuard<'a, T: ?Sized + 'a> {
    g: Option<RwSpinReadGuard<'a, T>>,
    m: &'a RwMutex<T>,
}

pub struct RwMutexWriteGuard<'a, T: ?Sized + 'a> {
    g: Option<RwSpinWriteGuard<'a, T>>,
    m: &'a RwMutex<T>,
}

pub struct RwMutexUpgradeableGuard<'a, T: ?Sized + 'a> {
    g: Option<RwSpinUpgradeableGuard<'a, T>>,
    m: &'a RwMutex<T>,
}

impl<'a, T: ?Sized> RwMutexUpgradeableGuard<'a, T> {
    pub fn upgrade(mut self) -> RwMutexWriteGuard<'a, T> {
        let task = current_task();

        self.m.reader_wait_queue.remove_task(task.clone());

        self.m.writer_wait_queue.add_task(task.clone());

        loop {
            match self.g.take().unwrap().try_upgrade() {
                Ok(l) => {
                    self.m.writer_wait_queue.remove_task(task);

                    return RwMutexWriteGuard::<'a, T> {
                        g: Some(l),
                        m: &self.m,
                    };
                }
                Err(u) => {
                    self.g = Some(u);

                    let _ = WaitQueue::task_wait();
                }
            }
        }
    }
}

impl<T: Default> Default for RwMutex<T> {
    fn default() -> Self {
        RwMutex::new(T::default())
    }
}

impl<T> RwMutex<T> {
    pub const fn new(user_data: T) -> RwMutex<T> {
        RwMutex::<T> {
            reader_wait_queue: WaitQueue::new(),
            writer_wait_queue: WaitQueue::new(),
            mutex: RwSpin::new_no_notify(user_data),
        }
    }

    pub fn reader_count(&self) -> usize {
        self.mutex.reader_count()
    }

    pub fn writer_count(&self) -> usize {
        self.mutex.writer_count()
    }

    pub fn read(&self) -> RwMutexReadGuard<T> {
        let task = current_task();

        self.reader_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_read() {
                self.reader_wait_queue.remove_task(task);
                return RwMutexReadGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    pub fn read_irq(&self) -> RwMutexReadGuard<T> {
        let task = current_task();

        self.reader_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_read_irq() {
                self.reader_wait_queue.remove_task(task);
                return RwMutexReadGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    pub fn read_upgradeable(&self) -> RwMutexUpgradeableGuard<T> {
        let task = current_task();

        self.reader_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_read_upgradeable() {
                self.reader_wait_queue.remove_task(task);
                return RwMutexUpgradeableGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    pub fn read_upgradeable_irq(&self) -> RwMutexUpgradeableGuard<T> {
        let task = current_task();

        self.reader_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_read_upgradeable_irq() {
                self.reader_wait_queue.remove_task(task);
                return RwMutexUpgradeableGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    pub fn write(&self) -> RwMutexWriteGuard<T> {
        let task = current_task();

        self.writer_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_write() {
                self.writer_wait_queue.remove_task(task);
                return RwMutexWriteGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }

    pub fn write_irq(&self) -> RwMutexWriteGuard<T> {
        let task = current_task();

        self.writer_wait_queue.add_task(task.clone());

        loop {
            if let Some(g) = self.mutex.try_write_irq() {
                self.writer_wait_queue.remove_task(task);
                return RwMutexWriteGuard::<T> {
                    g: Some(g),
                    m: &self,
                };
            } else {
                let _ = WaitQueue::task_wait();
            }
        }
    }
}

impl<'a, T: ?Sized> Deref for RwMutexReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwMutexUpgradeableGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwMutexWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for RwMutexWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for RwMutexReadGuard<'a, T> {
    fn drop(&mut self) {
        let _irq = IrqGuard::new();

        drop(self.g.take());

        if self.m.mutex.reader_count() == 0 {
            self.m.writer_wait_queue.notify_one();
        }
    }
}

impl<'a, T: ?Sized> Drop for RwMutexUpgradeableGuard<'a, T> {
    fn drop(&mut self) {
        let _irq = IrqGuard::new();

        drop(self.g.take());

        if self.m.mutex.reader_count() == 0 {
            self.m.writer_wait_queue.notify_one();
        }
    }
}

impl<'a, T: ?Sized> Drop for RwMutexWriteGuard<'a, T> {
    fn drop(&mut self) {
        let _irq = IrqGuard::new();

        drop(self.g.take());

        if self.m.mutex.writer_count() == 0 {
            self.m.reader_wait_queue.notify_all();
        }
    }
}
