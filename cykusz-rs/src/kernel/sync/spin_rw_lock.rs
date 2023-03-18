use core::ops::{Deref, DerefMut};

use crate::kernel;
use spin::{
    RwLock as RW, RwLockReadGuard as RWR, RwLockUpgradableGuard as RWU, RwLockWriteGuard as RWW,
};

use crate::kernel::int;

pub struct RwSpin<T: ?Sized> {
    notify: bool,
    l: RW<T>,
}

impl<T: Default> Default for RwSpin<T> {
    fn default() -> Self {
        RwSpin::new(T::default())
    }
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

pub struct RwSpinUpgradeableGuard<'a, T: ?Sized + 'a> {
    g: Option<RWU<'a, T>>,
    irq: bool,
    notify: bool,
}

impl<'a, T: ?Sized> RwSpinUpgradeableGuard<'a, T> {
    pub fn upgrade(mut self) -> RwSpinWriteGuard<'a, T> {
        let res = RwSpinWriteGuard {
            g: Some(self.g.take().unwrap().upgrade()),
            irq: self.irq,
            notify: self.notify,
        };

        core::mem::forget(self);

        res
    }

    pub fn try_upgrade(
        mut self,
    ) -> core::result::Result<RwSpinWriteGuard<'a, T>, RwSpinUpgradeableGuard<'a, T>> {
        match self.g.take().unwrap().try_upgrade() {
            Ok(l) => {
                let res = Ok(RwSpinWriteGuard {
                    g: Some(l),
                    irq: self.irq,
                    notify: self.notify,
                });

                core::mem::forget(self);

                res
            }
            Err(u) => {
                self.g = Some(u);

                Err(self)
            }
        }
    }
}

impl<T: ?Sized> RwSpin<T> {
    pub fn reader_count(&self) -> usize {
        self.l.reader_count()
    }

    pub fn writer_count(&self) -> usize {
        self.l.writer_count()
    }
}

impl<T> RwSpin<T> {
    pub const fn new(user_data: T) -> RwSpin<T> {
        RwSpin {
            notify: true,
            l: RW::new(user_data),
        }
    }

    pub const fn new_no_notify(user_data: T) -> RwSpin<T> {
        RwSpin {
            notify: false,
            l: RW::new(user_data),
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

    pub fn read(&self) -> RwSpinReadGuard<T> {
        let notify = self.maybe_preempt_disable();

        RwSpinReadGuard {
            g: Some(self.l.read()),
            irq: false,
            notify,
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

    pub fn read_upgradeable(&self) -> RwSpinUpgradeableGuard<T> {
        let notify = self.maybe_preempt_disable();
        RwSpinUpgradeableGuard {
            g: Some(self.l.upgradeable_read()),
            irq: false,
            notify,
        }
    }

    pub fn read_upgradeable_irq(&self) -> RwSpinUpgradeableGuard<T> {
        let ints = int::is_enabled();
        int::disable();
        RwSpinUpgradeableGuard {
            g: Some(self.l.upgradeable_read()),
            irq: ints,
            notify: false,
        }
    }

    pub fn try_read(&self) -> Option<RwSpinReadGuard<T>> {
        let notify = self.maybe_preempt_disable();

        let lock = match self.l.try_read() {
            Some(l) => Some(l),
            None => {
                if notify {
                    crate::kernel::sched::preempt_enable();
                }

                None
            }
        };

        if let Some(l) = lock {
            Some(RwSpinReadGuard {
                g: Some(l),
                irq: false,
                notify,
            })
        } else {
            None
        }
    }

    pub fn try_read_irq(&self) -> Option<RwSpinReadGuard<T>> {
        let ints = int::is_enabled();
        int::disable();
        let lock = match self.l.try_read() {
            Some(l) => Some(l),
            None => {
                if ints {
                    crate::kernel::int::enable();
                }

                None
            }
        };
        if let Some(l) = lock {
            Some(RwSpinReadGuard {
                g: Some(l),
                irq: ints,
                notify: false,
            })
        } else {
            None
        }
    }

    pub fn try_read_upgradeable(&self) -> Option<RwSpinUpgradeableGuard<T>> {
        let notify = self.maybe_preempt_disable();

        let lock = match self.l.try_upgradeable_read() {
            Some(l) => Some(l),
            None => {
                if notify {
                    crate::kernel::sched::preempt_enable();
                }

                None
            }
        };

        if let Some(l) = lock {
            Some(RwSpinUpgradeableGuard {
                g: Some(l),
                irq: false,
                notify,
            })
        } else {
            None
        }
    }

    pub fn try_read_upgradeable_irq(&self) -> Option<RwSpinUpgradeableGuard<T>> {
        let ints = int::is_enabled();
        int::disable();

        let lock = match self.l.try_upgradeable_read() {
            Some(l) => Some(l),
            None => {
                if ints {
                    crate::kernel::int::enable();
                }

                None
            }
        };

        if let Some(_l) = lock {
            Some(RwSpinUpgradeableGuard {
                g: Some(self.l.upgradeable_read()),
                irq: ints,
                notify: false,
            })
        } else {
            None
        }
    }

    pub fn write(&self) -> RwSpinWriteGuard<T> {
        let notify = self.maybe_preempt_disable();

        RwSpinWriteGuard {
            g: Some(self.l.write()),
            irq: false,
            notify,
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

    pub fn try_write(&self) -> Option<RwSpinWriteGuard<T>> {
        let notify = self.maybe_preempt_disable();

        let lock = match self.l.try_write() {
            Some(l) => Some(l),
            None => {
                if notify {
                    crate::kernel::sched::preempt_enable();
                }

                None
            }
        };

        if let Some(l) = lock {
            Some(RwSpinWriteGuard {
                g: Some(l),
                irq: false,
                notify,
            })
        } else {
            None
        }
    }

    pub fn try_write_irq(&self) -> Option<RwSpinWriteGuard<T>> {
        let ints = int::is_enabled();
        int::disable();
        let lock = match self.l.try_write() {
            Some(l) => Some(l),
            None => {
                if ints {
                    crate::kernel::int::enable();
                }

                None
            }
        };
        if let Some(l) = lock {
            Some(RwSpinWriteGuard {
                g: Some(l),
                irq: ints,
                notify: false,
            })
        } else {
            None
        }
    }
}

impl<'a, T: ?Sized> Deref for RwSpinReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwSpinUpgradeableGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> Deref for RwSpinWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.g.as_ref().unwrap()
    }
}

impl<'a, T: ?Sized> DerefMut for RwSpinWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.g.as_mut().unwrap()
    }
}

impl<'a, T: ?Sized> Drop for RwSpinReadGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.notify {
            crate::kernel::sched::preempt_enable();
        }
        if self.irq {
            int::enable();
        }
    }
}

impl<'a, T: ?Sized> Drop for RwSpinUpgradeableGuard<'a, T> {
    fn drop(&mut self) {
        drop(self.g.take());
        if self.notify {
            crate::kernel::sched::preempt_enable();
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
            crate::kernel::sched::preempt_enable();
        }
        if self.irq {
            int::enable();
        }
    }
}
