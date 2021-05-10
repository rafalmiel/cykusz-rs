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
}