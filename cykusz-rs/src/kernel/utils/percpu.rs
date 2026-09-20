use crate::kernel::ipi;
use crate::kernel::mm::heap::allocate_align;
use core::cell::UnsafeCell;
use core::ptr::Unique;

pub struct PerCpu<T> {
    data: UnsafeCell<Unique<T>>,
}

pub struct PerCpuIter<'a, T> {
    per_cpu: &'a PerCpu<T>,
    current: usize,
    target: ipi::IpiTarget,
}

impl<T: Default> Default for PerCpu<T> {
    fn default() -> Self {
        PerCpu::new_fn(|_| T::default())
    }
}

impl<'a, T> Iterator for PerCpuIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let cpu = self.current;
            self.current += 1;
            if cpu >= crate::kernel::smp::cpu_count() {
                return None
            } else {
                let is_valid = match self.target {
                    ipi::IpiTarget::All => true,
                    ipi::IpiTarget::Cpu(c) => c == cpu,
                    ipi::IpiTarget::This => cpu == crate::cpu_id() as usize,
                    ipi::IpiTarget::AllButThis => cpu != crate::cpu_id() as usize,
                };

                if is_valid {
                    return Some(self.per_cpu.cpu(cpu as isize))
                }
            }
        }
    }
}

impl<T> PerCpu<T> {
    pub const fn empty() -> PerCpu<T> {
        PerCpu::<T> {
            data: UnsafeCell::new(Unique::dangling()),
        }
    }

    pub fn new_fn(init: fn(usize) -> T) -> PerCpu<T> {
        use crate::kernel::smp::cpu_count;
        use ::core::mem::size_of;

        let mut this = PerCpu::<T>::empty();

        let cpu_count = cpu_count();

        let size = size_of::<T>() * cpu_count;
        let raw = allocate_align(size, align_of::<T>()).unwrap() as *mut T;
        assert_eq!(raw as usize % align_of::<T>(), 0, "PerCpu misaligned");

        unsafe {
            for i in 0..cpu_count {
                raw.offset(i as isize).write(init(i));
            }

            this.data = UnsafeCell::new(Unique::new_unchecked(raw));
        }

        this
    }

    unsafe fn ptr(&self) -> *mut T {
        unsafe { (&mut *self.data.get()).as_mut() }
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

    pub fn iter(&self) -> PerCpuIter<'_, T> {
        PerCpuIter {
            current: 0,
            per_cpu: self,
            target: ipi::IpiTarget::All,
        }
    }

    pub fn iter_ipi_target(&self, target: ipi::IpiTarget) -> PerCpuIter<'_, T> {
        PerCpuIter {
            current: 0,
            per_cpu: self,
            target,
        }
    }
}
