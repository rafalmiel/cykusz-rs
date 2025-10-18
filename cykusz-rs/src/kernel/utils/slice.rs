pub trait ToBytes {
    fn to_bytes(&self) -> &[u8];
    unsafe fn to_bytes_size(&self, size: usize) -> &[u8];
}

pub trait ToBytesMut {
    fn to_bytes_mut(&mut self) -> &mut [u8];
    unsafe fn to_bytes_size_mut(&mut self, size: usize) -> &mut [u8];
}

impl<T: Sized> ToBytes for &[T] {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self.as_ptr() as *const u8,
                self.len() * core::mem::size_of::<T>(),
            )
        }
    }
    unsafe fn to_bytes_size(&self, size: usize) -> &[u8] { unsafe {
        core::slice::from_raw_parts(self.as_ptr() as *const u8, size)
    }}
}

impl<T: Sized> ToBytesMut for &mut [T] {
    fn to_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.as_mut_ptr() as *mut u8,
                self.len() * core::mem::size_of::<T>(),
            )
        }
    }
    unsafe fn to_bytes_size_mut(&mut self, size: usize) -> &mut [u8] { unsafe {
        core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size)
    }}
}

impl<T: Sized> ToBytes for &T {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<T>())
        }
    }
    unsafe fn to_bytes_size(&self, size: usize) -> &[u8] { unsafe {
        core::slice::from_raw_parts(self as *const _ as *const u8, size)
    }}
}

impl<T: Sized> ToBytesMut for &mut T {
    fn to_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, core::mem::size_of::<T>())
        }
    }
    unsafe fn to_bytes_size_mut(&mut self, size: usize) -> &mut [u8] { unsafe {
        core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size)
    }}
}
