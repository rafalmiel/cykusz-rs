use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::kernel::int;

pub struct IrqLock<T: ?Sized> {
    l: UnsafeCell<T>,
}

pub struct IrqGuard {
    had_int: bool,
}

pub struct IrqLockGuard<'a, T: ?Sized + 'a> {
    data: &'a mut T,
    irq: bool,
}

unsafe impl<T: ?Sized + Send> Sync for IrqLock<T> {}

unsafe impl<T: ?Sized + Send> Send for IrqLock<T> {}

impl IrqGuard {
    pub fn new() -> IrqGuard {
        let g = IrqGuard {
            had_int: crate::kernel::int::is_enabled(),
        };

        crate::kernel::int::disable();

        g
    }

    pub fn maybe_new(irq: bool) -> IrqGuard {
        let g = IrqGuard { had_int: irq };

        if g.had_int {
            crate::kernel::int::disable();
        }

        g
    }
}

impl<T> IrqLock<T> {
    pub const fn new(user_data: T) -> IrqLock<T> {
        IrqLock {
            l: UnsafeCell::new(user_data),
        }
    }
}

impl<T: ?Sized> IrqLock<T> {
    pub fn irq(&self) -> IrqLockGuard<'_, T> {
        let ints = int::is_enabled();
        int::disable();
        IrqLockGuard {
            data: unsafe { &mut *self.l.get() },
            irq: ints,
        }
    }
}

impl<'a, T: ?Sized> Deref for IrqLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for IrqLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for IrqLockGuard<'a, T> {
    fn drop(&mut self) {
        if self.irq {
            int::enable();
        }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        if self.had_int {
            crate::kernel::int::enable();
        }
    }
}
