use crate::kernel;
pub use irq_lock::{IrqGuard, IrqLock, IrqLockGuard};
pub use mutex::{Mutex, MutexGuard};
pub use mutex_rw::{RwMutex, RwMutexReadGuard, RwMutexUpgradeableGuard, RwMutexWriteGuard};
pub use semaphore::Semaphore;
pub use spin_lock::{Spin, SpinGuard};
pub use spin_rw_lock::{RwSpin, RwSpinReadGuard, RwSpinWriteGuard};

mod irq_lock;
mod mutex;
mod mutex_rw;
mod semaphore;
mod spin_lock;
mod spin_rw_lock;

fn maybe_preempt_disable() -> bool {
    let notify = if kernel::int::is_enabled() {
        crate::kernel::sched::preempt_disable()
    } else {
        false
    };
    notify
}

pub trait LockGuard {}
pub trait LockApi<'a, T: ?Sized + 'a> {
    type Guard: LockGuard;

    fn lock(&'a self) -> Self::Guard;
    fn lock_debug(&'a self, _debug: usize) -> Self::Guard {
        self.lock()
    }
    fn try_lock(&'a self) -> Option<Self::Guard>;
    fn lock_irq(&'a self) -> Self::Guard;
    fn lock_irq_debug(&'a self, _debug: usize) -> Self::Guard {
        self.lock_irq()
    }
    fn try_lock_irq(&'a self) -> Option<Self::Guard>;
}
