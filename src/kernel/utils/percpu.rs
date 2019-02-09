use core::ptr::Unique;

pub struct PerCpu<T> {
    data: Unique<T>,
}

impl<T> PerCpu<T> {
    pub const fn empty() -> PerCpu<T> {
        PerCpu::<T> {
            data: ::core::ptr::Unique::empty(),
        }
    }

    pub fn init(&mut self) {
        use ::core::mem::size_of;
        use ::kernel::mm::heap::allocate;
        use ::kernel::smp::cpu_count;

        let size = size_of::<T>() * cpu_count();
        let raw = allocate(size).unwrap();

        unsafe {
            raw.write_bytes(0, size);
            self.data = ::core::ptr::Unique::new_unchecked(raw as *mut T);
        }

    }

    pub fn cpu(&self, cpu: isize) -> &T {
        unsafe {
            &*self.data.as_ptr().offset(cpu)
        }
    }

    pub fn cpu_mut(&self, cpu: isize) -> &mut T {
        unsafe {
            &mut *self.data.as_ptr().offset(cpu)
        }

    }

    pub fn this_cpu(&self) -> &T {
        self.cpu(unsafe {::CPU_ID} as isize)
    }

    pub fn this_cpu_mut(&self) -> &mut T {
        self.cpu_mut(unsafe {::CPU_ID} as isize)
    }
}