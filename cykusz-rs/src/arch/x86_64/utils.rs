pub struct StackHelper<'a> {
    ptr: &'a mut u64,
}

impl<'a> StackHelper<'a> {
    pub fn new(ptr: &'a mut u64) -> StackHelper<'a> {
        StackHelper::<'a> { ptr }
    }

    pub fn skip_by(&mut self, by: u64) {
        *self.ptr -= by;
    }

    pub unsafe fn write<T: Sized>(&mut self, val: T) {
        self.skip_by(core::mem::size_of::<T>() as u64);

        (*self.ptr as *mut T).write(val);
    }

    pub unsafe fn next<T: Sized>(&mut self) -> &mut T {
        self.skip_by(core::mem::size_of::<T>() as u64);

        &mut *(*self.ptr as *mut T)
    }

    pub fn restore_by(&mut self, by: u64) {
        *self.ptr += by;
    }

    pub unsafe fn restore<'b, T: Sized>(&mut self) -> &'b mut T {
        let v = &mut *(*self.ptr as *mut T);

        self.restore_by(core::mem::size_of::<T>() as u64);

        v
    }
}
