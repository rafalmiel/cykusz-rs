
bitflags! {
    pub struct PollEventFlags: usize {
        const READ          = 1 << 0;
        const WRITE         = 1 << 1;
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FdSet {
    pub fds: [u8; 128],
}

impl FdSet {
    pub fn new() -> FdSet {
        FdSet {
            fds: [0u8; 128],
        }
    }

    pub fn zero(&mut self) {
        self.fds.fill(0);
    }

    pub fn clear(&mut self, fd: usize) {
        self.fds[fd / 8] &= !(1 << (fd % 8));
    }

    pub fn is_set(&self, fd: usize) -> bool {
        self.fds[fd / 8] & (1 << (fd % 8)) > 0
    }

    pub fn set(&mut self, fd: usize) {
        self.fds[fd / 8] |= 1 << (fd % 8);
    }
}