use core::cell::UnsafeCell;
use core::ptr::Unique;

pub struct PerCpu<T> {
    data: UnsafeCell<Unique<T>>,
}

impl<T> PerCpu<T> {
    pub const fn empty() -> PerCpu<T> {
        PerCpu::<T> {
            data: UnsafeCell::new(::core::ptr::Unique::dangling()),
        }
    }

    pub fn new_fn(init: fn() -> T) -> PerCpu<T> {
        use crate::kernel::mm::heap::allocate;
        use crate::kernel::smp::cpu_count;
        use ::core::mem::size_of;

        let mut this = PerCpu::<T>::empty();

        let cpu_count = cpu_count();

        let size = size_of::<T>() * cpu_count;
        let raw = allocate(size).unwrap() as *mut T;

        unsafe {
            for i in 0..cpu_count {
                raw.offset(i as isize).write(init());
            }

            this.data = UnsafeCell::new(::core::ptr::Unique::new_unchecked(raw));
        }

        this
    }

    unsafe fn ptr(&self) -> *mut T {
        (&mut *self.data.get()).as_mut()
    }

    pub fn cpu(&self, cpu: isize) -> &T {
        unsafe { &*self.ptr().offset(cpu) }
    }

    pub fn cpu_mut(&self, cpu: isize) -> &mut T {
        unsafe { &mut *self.ptr().offset(cpu) }
    }

    pub fn this_cpu(&self) -> &T {
        self.cpu(unsafe { crate::CPU_ID } as isize)
    }

    pub fn this_cpu_mut(&self) -> &mut T {
        self.cpu_mut(unsafe { crate::CPU_ID } as isize)
    }
}
