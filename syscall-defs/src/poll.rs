bitflags! {
    pub struct PollEventFlags: u16 {
        const READ          = 1 << 0;
        const WRITE         = 1 << 1;
        const ERR           = 0x10;
        const HUP           = 0x08;
        const NVAL          = 0x40;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct FdSet {
    pub fds: [u8; 128],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollFd {
    pub fd: i32,
    pub events: PollEventFlags,
    pub revents: PollEventFlags,
}

impl FdSet {
    pub fn new() -> FdSet {
        FdSet { fds: [0u8; 128] }
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
