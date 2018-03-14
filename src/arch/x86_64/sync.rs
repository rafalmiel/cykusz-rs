use core::ops::{Deref, DerefMut};
use arch::int;
use spin::{Mutex as M, MutexGuard as MG};

pub struct Mutex<T> {
    l: M<T>,
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    g: Option<MG<'a, T>>,
    irq: bool,
}

impl<T> Mutex<T> {

    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            l: M::new(user_data),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        MutexGuard {
            g: Some(self.l.lock()),
            irq: false,
        }
    }

    pub fn lock_irq(&self) -> MutexGuard<T> {
        let ints = int::is_int_enabled();
        int::cli();
        MutexGuard {
            g: Some(self.l.lock()),
            //reenable ints if they were enabled before
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

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.irq {
            int::sti();
        }
    }
}