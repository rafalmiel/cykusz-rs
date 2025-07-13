use core::ops::{Deref, DerefMut};

use spin::{Mutex as M, MutexGuard as MG};

use crate::kernel::int;
use crate::kernel::sync::{IrqGuard, LockApi, LockGuard};

pub struct Spin<T: ?Sized> {
    notify: bool,
    l: M<T>,
}

impl<T: Default> Default for Spin<T> {
    fn default() -> Self {
        Spin::new(T::default())
    }
}

pub struct SpinGuard<'a, T: 'a + ?Sized> {
    g: Option<MG<'a, T>>,
    irq: bool,
    notify: bool,
    debug: usize,
}

impl<'a, T: 'a + ?Sized> LockGuard for SpinGuard<'a, T> {}

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
}

impl<'a, T: ?Sized + 'a> LockApi<'a, T> for Spin<T> {
    type Guard = SpinGuard<'a, T>;

    fn lock(&'a self) -> Self::Guard {
        let notify = self.notify && crate::kernel::sync::maybe_preempt_disable();

        let lock = self.l.lock();

        Self::Guard {
            g: Some(lock),
            irq: false,
            notify,
            debug: 0,
        }
    }

    fn lock_debug(&'a self, id: usize) -> Self::Guard {
        let notify = self.notify && crate::kernel::sync::maybe_preempt_disable();

        dbgln!(lock, "l: - {}", id);
        let lock = self.l.lock();
        dbgln!(lock, "l: + {}", id);

        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: id,
        }
    }

    fn try_lock(&'a self) -> Option<Self::Guard> {
        let notify = self.notify && crate::kernel::sync::maybe_preempt_disable();

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
            Some(Self::Guard {
                g: Some(g),
                irq: false,
                notify,
                debug: 0,
            })
        } else {
            None
        }
    }

    fn lock_irq(&'a self) -> Self::Guard {
        let int_enabled = crate::kernel::int::is_enabled();
        crate::kernel::int::disable();

        let lock = self.l.lock();

        Self::Guard {
            g: Some(lock),
            irq: int_enabled,
            notify: false,
            debug: 0,
        }
    }

    fn lock_irq_debug(&'a self, id: usize) -> Self::Guard {
        let int_enabled = crate::kernel::int::is_enabled();
        crate::kernel::int::disable();

        dbgln!(lock, "l: - {}", id);
        let lock = self.l.lock();
        dbgln!(lock, "l: + {}", id);

        SpinGuard {
            g: Some(lock),
            irq: int_enabled,
            notify: false,
            debug: id,
        }
    }

    fn try_lock_irq(&'a self) -> Option<Self::Guard> {
        let int_enabled = crate::kernel::int::is_enabled();
        crate::kernel::int::disable();

        let lock = match self.l.try_lock() {
            Some(l) => Some(l),
            None => {
                if int_enabled {
                    crate::kernel::int::enable();
                }

                None
            }
        };

        if let Some(g) = lock {
            Some(Self::Guard {
                g: Some(g),
                irq: int_enabled,
                notify: false,
                debug: 0,
            })
        } else {
            None
        }
    }
}

impl Spin<()> {
    pub fn unguarded_release(&self) {
        unsafe {
            self.l.force_unlock();
        }
    }

    pub fn unguarded_obtain(&self) {
        spin::MutexGuard::leak(self.l.lock());
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

impl<'a, T: ?Sized> SpinGuard<'a, T> {
    pub fn to_irq_guard(mut self) -> IrqGuard {
        let g = IrqGuard::maybe_new(self.irq);

        self.irq = false;

        g
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.debug > 0 {
            dbgln!(lock, "l: D {}", self.debug)
        }
        if self.notify {
            crate::kernel::sched::preempt_enable();
        }
        if self.irq {
            int::enable();
        }
    }
}
