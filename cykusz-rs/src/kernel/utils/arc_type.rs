use alloc::sync::{Arc, Weak};
use core::mem::ManuallyDrop;
use core::ops::Deref;

pub trait Uid {
    fn uid(&self) -> usize;
}

pub struct ArcType<T: ?Sized + Uid>(Arc<T>);
#[derive(Default)]
pub struct WeakType<T: ?Sized + Uid>(Weak<T>);

impl<T: ?Sized + Uid> Clone for ArcType<T> {
    fn clone(&self) -> Self {
        let cloned = ArcType::<T>(self.0.clone());
        cloned.log_plus();
        cloned
    }
}
impl<T: ?Sized + Uid> Clone for WeakType<T> {
    fn clone(&self) -> Self {
        WeakType::<T>(self.0.clone())
    }
}

impl<T: ?Sized + Uid> Drop for ArcType<T> {
    fn drop(&mut self) {
        self.log_minus();
    }
}

unsafe impl<T: ?Sized + Uid> Send for ArcType<T> {}
unsafe impl<T: ?Sized + Uid> Sync for ArcType<T> {}

impl<T: ?Sized + Uid> Deref for ArcType<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: ?Sized + Uid> From<Arc<T>> for ArcType<T> {
    fn from(value: Arc<T>) -> Self {
        ArcType::<T>(value)
    }
}

impl<T: ?Sized + Uid> From<Weak<T>> for WeakType<T> {
    fn from(value: Weak<T>) -> Self {
        WeakType::<T>(value)
    }
}

impl<T: Uid> ArcType<T> {
    pub fn new(d: Arc<T>) -> ArcType<T> {
        ArcType::<T>(d)
    }
    pub fn new_cyclic<F>(data_fn: F) -> ArcType<T>
    where
        F: FnOnce(&Weak<T>) -> T,
    {
        ArcType::<T>(Arc::new_cyclic(data_fn))
    }
    pub fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
    pub fn strong_count(task: &ArcType<T>) -> usize {
        Arc::strong_count(&task.0)
    }
    pub fn weak_count(task: &ArcType<T>) -> usize {
        Arc::weak_count(&task.0)
    }
    pub unsafe fn decrement_strong_count(ptr: *const T) { unsafe {
        Arc::decrement_strong_count(ptr)
    }}
}

impl<T: ?Sized + Uid> ArcType<T> {
    pub fn into_raw(this: Self) -> *const T {
        let this = ManuallyDrop::new(this);
        Arc::as_ptr(&this.0)
    }

    pub fn log_plus(&self) {}

    pub fn log_minus(&self) {}
}

impl<T: ?Sized + Uid> WeakType<T> {
    pub fn upgrade(&self) -> Option<ArcType<T>> {
        let upgraded = ArcType::<T>(self.0.upgrade()?);
        upgraded.log_plus();
        Some(upgraded)
    }
}
