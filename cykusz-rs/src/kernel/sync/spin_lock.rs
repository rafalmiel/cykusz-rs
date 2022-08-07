use core::ops::{Deref, DerefMut};

use spin::{Mutex as M, MutexGuard as MG};

use crate::kernel::int;
use crate::kernel::sched::current_id;

pub struct Spin<T: ?Sized> {
    notify: bool,
    l: M<T>,
}

impl<T: Default> Default for Spin<T> {
    fn default() -> Self {
        Spin::new(T::default())
    }
}

pub struct SpinGuard<'a, T: ?Sized + 'a> {
    g: Option<MG<'a, T>>,
    irq: bool,
    notify: bool,
    debug: usize,
}

impl<T> Spin<T> {
    pub const fn new(user_data: T) -> Spin<T> {
        Spin {
            notify: true,
            l: M::new(user_data),
        }
    }

    pub const fn new_no_notify(user_data: T) -> Spin<T> {
        Spin {
            notify: false,
            l: M::new(user_data),
        }
    }

    pub fn lock(&self) -> SpinGuard<T> {
        let notify = if self.notify {
            crate::kernel::sched::preempt_disable()
        } else {
            false
        };

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: 0,
        }
    }

    pub fn lock_debug(&self, _id: usize) -> SpinGuard<T> {
        self.lock()
    }

    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        let notify = if self.notify {
            crate::kernel::sched::preempt_disable()
        } else {
            false
        };

        let lock = match self.l.try_lock() {
            Some(l) => Some(l),
            None => {
                if notify {
                    crate::kernel::sched::preempt_enable();
                }

                None
            }
        };

        if let Some(g) = lock {
            Some(SpinGuard {
                g: Some(g),
                irq: false,
                notify,
                debug: 0,
            })
        } else {
            None
        }
    }

    pub fn try_lock_irq(&self) -> Option<SpinGuard<T>> {
        let int_enabled = crate::kernel::int::is_enabled();

        crate::kernel::int::disable();

        let notify = if self.notify {
            //crate::kernel::sched::preempt_disable()
            false
        } else {
            false
        };

        let lock = match self.l.try_lock() {
            Some(l) => Some(l),
            None => {
                if notify {
                    crate::kernel::sched::preempt_enable();
                }

                if int_enabled {
                    crate::kernel::int::enable();
                }

                None
            }
        };

        if let Some(g) = lock {
            Some(SpinGuard {
                g: Some(g),
                irq: int_enabled,
                notify,
                debug: 0,
            })
        } else {
            None
        }
    }

    pub fn lock_irq(&self) -> SpinGuard<T> {
        let int_enabled = crate::kernel::int::is_enabled();

        crate::kernel::int::disable();

        let notify = if self.notify {
            //crate::kernel::sched::preempt_disable()
            false
        } else {
            false
        };

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: int_enabled,
            notify,
            debug: 0,
        }
    }

    pub fn lock_irq_debug(&self, _id: usize) -> SpinGuard<T> {
        let int_enabled = crate::kernel::int::is_enabled();

        crate::kernel::int::disable();

        let notify = if self.notify {
            //crate::kernel::sched::preempt_disable()
            false
        } else {
            false
        };

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: int_enabled,
            notify,
            debug: _id,
        }
    }
}

impl Spin<()> {
    pub fn unguarded_release(&self) {
        spin::MutexGuard::leak(self.l.lock());
    }

    pub fn unguarded_obtain(&self) {
        unsafe {
            self.l.force_unlock();
        }
    }
}

impl<'a, T: ?Sized> Deref for SpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for SpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.debug > 0 {
            logln!("U {} {}", self.debug, current_id());
        }
        if self.notify {
            crate::kernel::sched::preempt_enable();
        }
        if self.irq {
            int::enable();
        }
    }
}
