use core::cell::UnsafeCell;
use core::default::Default;
use core::fmt;
use core::hint::spin_loop as cpu_relax;
use core::marker::Sync;
use core::ops::{Deref, DerefMut, Drop};
use core::option::Option::{self, None, Some};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// This type provides MUTual EXclusion based on spinning.
///
/// # Description
///
/// The behaviour of these lock is similar to their namesakes in `std::sync`. they
/// differ on the following:
///
/// - The lock will not be poisoned in case of failure;
///
/// # Simple examples
///
/// ```
/// use spin;
/// let spin_mutex = spin::Mutex::new(0);
///
/// // Modify the data
/// {
///     let mut data = spin_mutex.lock();
///     *data = 2;
/// }
///
/// // Read the data
/// let answer =
/// {
///     let data = spin_mutex.lock();
///     *data
/// };
///
/// assert_eq!(answer, 2);
/// ```
///
/// # Thread-safety example
///
/// ```
/// use spin;
/// use std::sync::{Arc, Barrier};
///
/// let numthreads = 1000;
/// let spin_mutex = Arc::new(spin::Mutex::new(0));
///
/// // We use a barrier to ensure the readout happens after all writing
/// let barrier = Arc::new(Barrier::new(numthreads + 1));
///
/// for _ in (0..numthreads)
/// {
///     let my_barrier = barrier.clone();
///     let my_lock = spin_mutex.clone();
///     std::thread::spawn(move||
///     {
///         let mut guard = my_lock.lock();
///         *guard += 1;
///
///         // Release the lock to prevent a deadlock
///         drop(guard);
///         my_barrier.wait();
///     });
/// }
///
/// barrier.wait();
///
/// let answer = { *spin_mutex.lock() };
/// assert_eq!(answer, numthreads);
/// ```
pub struct RawSpin<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
#[derive(Debug)]
pub struct RawSpinGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for RawSpin<T> {}

unsafe impl<T: ?Sized + Send> Send for RawSpin<T> {}

impl<T> RawSpin<T> {
    /// Creates a new spinlock wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```
    /// use spin;
    ///
    /// static MUTEX: spin::Mutex<()> = spin::Mutex::new(());
    ///
    /// fn demo() {
    ///     let lock = MUTEX.lock();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub const fn new(user_data: T) -> RawSpin<T> {
        RawSpin {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(user_data),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let RawSpin { data, .. } = self;
        data.into_inner()
    }
}

impl<T: ?Sized> RawSpin<T> {
    fn obtain_lock(&self) {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_err()
        {
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                cpu_relax();
            }
        }
    }
    fn obtain_lock_with_int(&self) {
        let had_int = crate::kernel::int::is_enabled();

        if !had_int {
            self.obtain_lock();
            return;
        }

        crate::kernel::int::disable();

        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_err()
        {
            crate::kernel::int::enable();
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                cpu_relax();
            }
            crate::kernel::int::disable();
        }
    }

    fn obtain_lock_with_ref(&self, locks: &AtomicUsize) {
        let had_int = crate::kernel::int::is_enabled();

        if !had_int {
            self.obtain_lock();
            locks.fetch_add(1, Ordering::SeqCst);
            return;
        }

        crate::kernel::int::disable();
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_err()
        {
            crate::kernel::int::enable();
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                cpu_relax();
            }
            crate::kernel::int::disable();
        }
        locks.fetch_add(1, Ordering::SeqCst);

        crate::kernel::int::enable();
    }

    fn release_lock(&self) {
        self.lock.store(false, Ordering::Relaxed)
    }

    /// Locks the spinlock and returns a guard.
    ///
    /// The returned value may be dereferenced for data access
    /// and the lock will be dropped when the guard falls out of scope.
    ///
    /// ```
    /// let mylock = spin::Mutex::new(0);
    /// {
    ///     let mut data = mylock.lock();
    ///     // The lock is now locked and the data can be accessed
    ///     *data += 1;
    ///     // The lock is implicitly dropped
    /// }
    ///
    /// ```
    pub fn lock(&self) -> RawSpinGuard<T> {
        self.obtain_lock();
        RawSpinGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }
    pub fn lock_with_int(&self) -> RawSpinGuard<T> {
        self.obtain_lock_with_int();
        RawSpinGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }
    pub fn lock_with_ref(&self, locks: &AtomicUsize) -> RawSpinGuard<T> {
        self.obtain_lock_with_ref(locks);
        RawSpinGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }

    /// Force unlock the spinlock.
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread. However, this can be useful in some instances for exposing the
    /// lock to FFI that doesn't know how to deal with RAII.
    ///
    /// If the lock isn't held, this is a no-op.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    /// Tries to lock the mutex. If it is already locked, it will return None. Otherwise it returns
    /// a guard within Some.
    pub fn try_lock(&self) -> Option<RawSpinGuard<T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_ok()
        {
            Some(RawSpinGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn try_lock_with_int(&self) -> Option<RawSpinGuard<T>> {
        let had_int = crate::kernel::int::is_enabled();

        if !had_int {
            return self.try_lock();
        }

        crate::kernel::int::disable();

        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_ok()
        {
            Some(RawSpinGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            crate::kernel::int::enable();

            None
        }
    }

    pub fn try_lock_with_ref(&self, locks: &AtomicUsize) -> Option<RawSpinGuard<T>> {
        let had_int = crate::kernel::int::is_enabled();

        if !had_int {
            let res = self.try_lock();

            if res.is_some() {
                locks.fetch_add(1, Ordering::SeqCst);
            }

            return res;
        }

        crate::kernel::int::disable();

        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_ok()
        {
            locks.fetch_add(1, Ordering::SeqCst);

            crate::kernel::int::enable();

            Some(RawSpinGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            crate::kernel::int::enable();

            None
        }
    }
}

impl RawSpin<()> {
    pub fn unguarded_release(&self) {
        self.release_lock()
    }

    pub fn unguarded_obtain(&self) {
        self.obtain_lock()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RawSpin<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for RawSpin<T> {
    fn default() -> RawSpin<T> {
        RawSpin::new(Default::default())
    }
}

impl<'a, T: ?Sized> Deref for RawSpinGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for RawSpinGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for RawSpinGuard<'a, T> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}
