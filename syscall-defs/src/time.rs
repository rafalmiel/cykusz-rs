#[repr(C)]
#[derive(Default, Debug)]
pub struct Timespec {
    pub secs: u64,
    pub nsecs: u64,
}

impl Timespec {
    pub fn from_secs(secs: usize) -> Timespec {
        Timespec {
            secs: secs as u64,
            nsecs: 0,
        }
    }

    pub fn to_nanoseconds(&self) -> usize {
        self.secs as usize * 1000000000usize + self.nsecs as usize
    }
}
