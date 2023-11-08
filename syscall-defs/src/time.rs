pub const UTIME_NOW: u64 = (1u64 << 30) - 1;
pub const UTIME_OMIT: u64 = (1u64 << 30) - 2;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
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

    pub fn is_now(&self) -> bool {
        self.nsecs == UTIME_NOW
    }

    pub fn is_omit(&self) -> bool {
        self.nsecs == UTIME_OMIT
    }
}
