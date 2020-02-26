use core::ops::{Deref, DerefMut};

use crate::kernel::sync::raw_spin::{RawSpin as M, RawSpinGuard as MG};

use crate::kernel::int;

pub struct Spin<T: ?Sized> {
    l: M<T>,
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
        crate::kernel::sched::enter_critical_section();
        SpinGuard {
            g: Some(self.l.lock()),
            irq: false,
            notify: true,
            debug: 0,
        }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        if let Some(g) = self.l.try_lock() {
            crate::kernel::sched::enter_critical_section();

            Some(SpinGuard {
                g: Some(g),
                irq: false,
                notify: true,
                debug: 0
            })
        } else {
            None
        }
    }

    pub fn try_lock_irq(&self) -> Option<SpinGuard<T>> {
        if let Some(g) = self.l.try_lock() {
            let ints = int::is_enabled();
            int::disable();
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
        crate::kernel::sched::enter_critical_section();

        println!("-{} ints: {}", id, crate::kernel::int::is_enabled());
        let l = self.l.lock();
        println!("+{}", id);

        SpinGuard {
            g: Some(l),
            irq: false,
            notify: true,
            debug: id,
        }
    }

    pub fn lock_irq(&self) -> SpinGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        SpinGuard {
            g: Some(self.l.lock()),
            //reenable ints if they were enabled before
            irq: ints,
            notify: false,
            debug: 0,
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
    fn deref<'b>(&'b self) -> &'b T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for SpinGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
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

