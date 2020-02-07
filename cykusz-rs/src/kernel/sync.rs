use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use spin::{Mutex as M, MutexGuard as MG};
use spin::{RwLock as RW, RwLockReadGuard as RWR, RwLockWriteGuard as RWW};

use crate::kernel::int;

pub struct Mutex<T> {
    l: M<T>,
}

pub struct RwLock<T> {
    l: RW<T>,
}

pub struct IrqLock<T: ?Sized> {
    l: UnsafeCell<T>,
}

pub struct IrqGuard {
    had_int: bool,
}

unsafe impl<T: ?Sized + Send> Sync for IrqLock<T> {}

unsafe impl<T: ?Sized + Send> Send for IrqLock<T> {}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    g: Option<MG<'a, T>>,
    irq: bool,
    notify: bool,
    debug: usize,
}

pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    g: Option<RWR<'a, T>>,
    irq: bool,
    notify: bool,
}

pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    g: Option<RWW<'a, T>>,
    irq: bool,
    notify: bool,
}

pub struct IrqLockGuard<'a, T: ?Sized + 'a> {
    data: &'a mut T,
    irq: bool,
}

impl IrqGuard {
    pub fn new() -> IrqGuard {
        let g = IrqGuard {
            had_int: crate::kernel::int::is_enabled(),
        };

        crate::kernel::int::disable();

        g
    }
}

impl<T> Mutex<T> {
    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            l: M::new(user_data),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        crate::kernel::sched::enter_critical_section();
        MutexGuard {
            g: Some(self.l.lock()),
            irq: false,
            notify: true,
            debug: 0,
        }
    }

    pub fn lock_debug(&self, id: usize) -> MutexGuard<T> {
        crate::kernel::sched::enter_critical_section();

        println!("-{} ints: {}", id, crate::kernel::int::is_enabled());
        let l = self.l.lock();
        println!("+{}", id);

        MutexGuard {
            g: Some(l),
            irq: false,
            notify: true,
            debug: id,
        }
    }

    pub fn lock_irq(&self) -> MutexGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        MutexGuard {
            g: Some(self.l.lock()),
            //reenable ints if they were enabled before
            irq: ints,
            notify: false,
            debug: 0,
        }
    }
}

impl<T> RwLock<T> {
    pub const fn new(user_data: T) -> RwLock<T> {
        RwLock {
            l: RW::new(user_data),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        crate::kernel::sched::enter_critical_section();
        RwLockReadGuard {
            g: Some(self.l.read()),
            irq: false,
            notify: true,
        }
    }

    pub fn read_irq(&self) -> RwLockReadGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        RwLockReadGuard {
            g: Some(self.l.read()),
            irq: ints,
            notify: false,
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        crate::kernel::sched::enter_critical_section();
        RwLockWriteGuard {
            g: Some(self.l.write()),
            irq: false,
            notify: true,
        }
    }

    pub fn write_irq(&self) -> RwLockWriteGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        RwLockWriteGuard {
            g: Some(self.l.write()),
            irq: ints,
            notify: false,
        }
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

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.debug > 0 {
            println!("U {}", self.debug);
        }
        if self.notify {
            crate::kernel::sched::leave_critical_section();
        }
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

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
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

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
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

impl Drop for IrqGuard {
    fn drop(&mut self) {
        if self.had_int {
            crate::kernel::int::enable();
        }
    }
}
