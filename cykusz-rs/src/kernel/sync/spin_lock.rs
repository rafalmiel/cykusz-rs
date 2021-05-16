use core::ops::{Deref, DerefMut};

use crate::kernel::int;
use crate::kernel::sched::{current_id, current_locks_var};
use crate::kernel::sync::raw_spin::{RawSpin as M, RawSpinGuard as MG};

pub struct Spin<T: ?Sized> {
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
            l: M::new(user_data),
        }
    }

    pub fn lock(&self) -> SpinGuard<T> {
        let (lock, notify) = if let Some(locks) = current_locks_var() {
            (self.l.lock_with_ref(locks), true)
        } else {
            (self.l.lock(), false)
        };
        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: 0,
        }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        let (lock, notify) = if let Some(locks) = current_locks_var() {
            (self.l.try_lock_with_ref(locks), true)
        } else {
            (self.l.try_lock(), false)
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
        let ints = int::is_enabled();
        if let Some(g) = self.l.try_lock_with_int() {
            Some(SpinGuard {
                g: Some(g),
                //reenable ints if they were enabled before
                irq: ints,
                notify: false,
                debug: 0,
            })
        } else {
            None
        }
    }

    pub fn lock_debug(&self, id: usize) -> SpinGuard<T> {
        logln!(
            "-{} ints: {} {}",
            id,
            crate::kernel::int::is_enabled(),
            current_id()
        );
        let (lock, notify) = if let Some(locks) = current_locks_var() {
            (self.l.lock_with_ref(locks), true)
        } else {
            (self.l.lock(), false)
        };
        logln!("+{} {}", id, current_id());

        SpinGuard {
            g: Some(lock),
            irq: false,
            notify,
            debug: id,
        }
    }

    pub fn lock_irq(&self) -> SpinGuard<T> {
        let ints = int::is_enabled();
        let lock = self.l.lock_with_int();
        SpinGuard {
            g: Some(lock),
            //reenable ints if they were enabled before
            irq: ints,
            notify: false,
            debug: 0,
        }
    }

    pub fn lock_irq_debug(&self, id: usize) -> SpinGuard<T> {
        let ints = int::is_enabled();
        logln!(
            "-{} ints: {} {}",
            id,
            crate::kernel::int::is_enabled(),
            current_id()
        );
        let l = self.l.lock_with_int();
        logln!("+{} {}", id, current_id());
        SpinGuard {
            g: Some(l),
            //reenable ints if they were enabled before
            irq: ints,
            notify: false,
            debug: id,
        }
    }
}

impl Spin<()> {
    pub fn unguarded_release(&self) {
        self.l.unguarded_release()
    }

    pub fn unguarded_obtain(&self) {
        self.l.unguarded_obtain()
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
        if self.irq {
            int::enable();
        }
        if self.notify {
            crate::kernel::sched::leave_critical_section();
        }
    }
}
