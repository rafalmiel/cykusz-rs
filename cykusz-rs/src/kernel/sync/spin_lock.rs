use core::ops::{Deref, DerefMut};

use crate::kernel;
use spin::{Mutex as M, MutexGuard as MG};

use crate::kernel::int;
use crate::kernel::sync::IrqGuard;

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

    fn maybe_preempt_disable(&self) -> bool {
        let notify = if self.notify && kernel::int::is_enabled() {
            crate::kernel::sched::preempt_disable()
        } else {
            false
        };
        notify
    }

    pub fn lock(&self) -> SpinGuard<T> {
        let notify = self.maybe_preempt_disable();

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: 0,
        }
    }

    pub fn lock_debug(&self, id: usize) -> SpinGuard<T> {
        let notify = self.maybe_preempt_disable();

        logln!("l: - {}", id);
        let lock = self.l.lock();
        logln!("l: + {}", id);

        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: id,
        }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        let notify = self.maybe_preempt_disable();

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
            Some(SpinGuard {
                g: Some(g),
                irq: int_enabled,
                notify: false,
                debug: 0,
            })
        } else {
            None
        }
    }

    pub fn lock_irq(&self) -> SpinGuard<T> {
        let int_enabled = crate::kernel::int::is_enabled();
        crate::kernel::int::disable();

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: int_enabled,
            notify: false,
            debug: 0,
        }
    }

    pub fn lock_irq_debug(&self, id: usize) -> SpinGuard<T> {
        let int_enabled = crate::kernel::int::is_enabled();
        crate::kernel::int::disable();

        let lock = self.l.lock();

        SpinGuard {
            g: Some(lock),
            irq: int_enabled,
            notify: false,
            debug: id,
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
        if self.debug > 0 {}
        if self.notify {
            crate::kernel::sched::preempt_enable();
        }
        if self.irq {
            int::enable();
        }
    }
}
