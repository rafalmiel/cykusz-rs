use alloc::vec::Vec;

pub struct BufBlock {
    block: usize,
    buf: Vec<u8>,
}

impl BufBlock {
    pub fn new(size: usize) -> BufBlock {
        let mut vec = Vec::<u8>::new();
        vec.resize(size, 0);

        BufBlock { block: 0, buf: vec }
    }

    pub fn empty() -> BufBlock {
        BufBlock {
            block: 0,
            buf: Vec::new(),
        }
    }

    pub fn set_block(&mut self, nr: usize) {
        self.block = nr;
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn bytes(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}
