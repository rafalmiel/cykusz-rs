use core::ops::{Deref, DerefMut};
use kernel::int;
use spin::{Mutex as M, MutexGuard as MG};
use core::cell::UnsafeCell;

pub struct Mutex<T> {
    l: M<T>,
}

pub struct IrqLock<T: ?Sized> {
    l: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for IrqLock<T> {}
unsafe impl<T: ?Sized + Send> Send for IrqLock<T> {}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    g: Option<MG<'a, T>>,
    irq: bool,
}

pub struct IrqLockGuard<'a, T: ?Sized + 'a> {
    data: &'a mut T,
    irq: bool,
}

impl<T> Mutex<T> {

    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            l: M::new(user_data),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        ::kernel::sched::enter_critical_section();
        MutexGuard {
            g: Some(self.l.lock()),
            irq: false,
        }
    }

    pub fn lock_irq(&self) -> MutexGuard<T> {
        let ints = int::is_enabled();
        ::kernel::sched::enter_critical_section();
        int::disable();
        MutexGuard {
            g: Some(self.l.lock()),
            //reenable ints if they were enabled before
            irq: ints,
        }
    }
}

impl<T> IrqLock<T> {
    pub const fn new(user_data: T) -> IrqLock<T> {
        IrqLock {
            l: UnsafeCell::new(user_data)
        }
    }
}

impl<T:?Sized> IrqLock<T> {
    pub fn irq(&self) -> IrqLockGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        IrqLockGuard {
            data: unsafe { &mut *self.l.get() },
            irq: ints,
        }
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for IrqLockGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for IrqLockGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        ::kernel::sched::leave_critical_section();
        if self.irq {
            int::enable();
        }
    }
}

impl<'a, T: ?Sized> Drop for IrqLockGuard<'a, T> {
    fn drop(&mut self) {
        if self.irq {
            int::enable();
        }
    }
}
