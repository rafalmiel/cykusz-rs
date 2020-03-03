use core::ops::{Deref, DerefMut};

use spin::{RwLock as RW, RwLockReadGuard as RWR, RwLockWriteGuard as RWW};

use crate::kernel::int;

pub struct RwSpin<T> {
    l: RW<T>,
}

pub struct RwSpinReadGuard<'a, T: ?Sized + 'a> {
    g: Option<RWR<'a, T>>,
    irq: bool,
    notify: bool,
}

pub struct RwSpinWriteGuard<'a, T: ?Sized + 'a> {
    g: Option<RWW<'a, T>>,
    irq: bool,
    notify: bool,
}

impl<T> RwSpin<T> {
    pub const fn new(user_data: T) -> RwSpin<T> {
        RwSpin {
            l: RW::new(user_data),
        }
    }

    pub fn read(&self) -> RwSpinReadGuard<T> {
        crate::kernel::sched::enter_critical_section();
        RwSpinReadGuard {
            g: Some(self.l.read()),
            irq: false,
            notify: true,
        }
    }

    pub fn read_irq(&self) -> RwSpinReadGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        RwSpinReadGuard {
            g: Some(self.l.read()),
            irq: ints,
            notify: false,
        }
    }

    pub fn write(&self) -> RwSpinWriteGuard<T> {
        crate::kernel::sched::enter_critical_section();
        RwSpinWriteGuard {
            g: Some(self.l.write()),
            irq: false,
            notify: true,
        }
    }

    pub fn write_irq(&self) -> RwSpinWriteGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        RwSpinWriteGuard {
            g: Some(self.l.write()),
            irq: ints,
            notify: false,
        }
    }
}

impl<'a, T: ?Sized> Deref for RwSpinReadGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwSpinWriteGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for RwSpinWriteGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for RwSpinReadGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.notify {
            crate::kernel::sched::leave_critical_section();
        }
        if self.irq {
            int::enable();
        }
    }
}

impl<'a, T: ?Sized> Drop for RwSpinWriteGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.notify {
            crate::kernel::sched::leave_critical_section();
        }
        if self.irq {
            int::enable();
        }
    }
}
