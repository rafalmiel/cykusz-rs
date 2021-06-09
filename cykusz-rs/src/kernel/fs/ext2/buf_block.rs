use alloc::vec::Vec;

pub struct SliceBlock<T: Sized + Default + Copy> {
    block: usize,
    buf: Vec<T>,
}

impl<T: Sized + Default + Copy> SliceBlock<T> {
    pub fn new(size: usize) -> SliceBlock<T> {
        let mut vec = Vec::<T>::new();
        vec.resize(size, T::default());

        SliceBlock::<T> { block: 0, buf: vec }
    }

    pub fn empty() -> SliceBlock<T> {
        SliceBlock::<T> {
            block: 0,
            buf: Vec::new(),
        }
    }

    pub fn alloc(&mut self, size: usize) {
        self.buf.resize(size, T::default());
    }

    pub fn clear(&mut self) {
        self.buf.clear()
    }

    pub fn set_block(&mut self, nr: usize) {
        self.block = nr;
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn slice(&self) -> &[T] {
        self.buf.as_slice()
    }

    pub fn slice_mut(&mut self) -> &mut [T] {
        self.buf.as_mut_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

pub type BufBlock = SliceBlock<u8>;

impl SliceBlock<u8> {
    pub fn bytes(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }
}
