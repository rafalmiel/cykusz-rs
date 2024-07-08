bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct WaitPidFlags: usize {
        const NOHANG        = 1 << 0;
        const UNTRACED      = 1 << 1;
        const STOPPED       = 1 << 1;
        const EXITED        = 1 << 2;
        const CONTINUED     = 1 << 3;
        const NOWAIT        = 0x01000000;
    }
}

impl WaitPidFlags {
    pub fn nohang(&self) -> bool {
        self.contains(WaitPidFlags::NOHANG)
    }

    pub fn stopped(&self) -> bool {
        self.contains(WaitPidFlags::STOPPED)
    }

    pub fn continued(&self) -> bool {
        self.contains(WaitPidFlags::CONTINUED)
    }

    pub fn exited(&self) -> bool {
        self.contains(WaitPidFlags::EXITED) || self.is_empty()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Status {
    Exited(u64),
    Signaled(u64),
    Stopped(u64),
    Continued,
    Invalid(u64),
}

impl Status {
    pub fn is_exited(&self) -> bool {
        if let Status::Exited(_) = self {
            return true;
        }

        false
    }
    pub fn is_continued(&self) -> bool {
        if let Status::Continued = self {
            return true;
        }

        false
    }

    pub fn is_stopped(&self) -> bool {
        if let Status::Stopped(_) = self {
            return true;
        }

        false
    }
    pub fn is_signaled(&self) -> bool {
        if let Status::Signaled(_) = self {
            return true;
        }

        false
    }

    pub fn is_invalid(&self) -> bool {
        if let Status::Invalid(_) = self {
            return true;
        }

        false
    }

    pub fn which_signal(&self) -> u64 {
        if let Status::Signaled(s) = self {
            return *s;
        }

        panic!("not signaled");
    }
}

pub struct RawStatus(u64);

impl RawStatus {
    pub fn exit_status(&self) -> u64 {
        (self.0 & 0xff00) >> 8
    }

    pub fn term_sig(&self) -> u64 {
        self.0 & 0x7f
    }

    pub fn stop_sig(&self) -> u64 {
        self.exit_status()
    }

    pub fn is_exited(&self) -> bool {
        self.term_sig() == 0
    }

    pub fn is_signaled(&self) -> bool {
        ((((self.0 & 0x7f) + 1) as i8) >> 1) > 0
    }

    pub fn is_stopped(&self) -> bool {
        (self.0 & 0xff) == 0x7f
    }

    pub fn is_continued(&self) -> bool {
        self.0 == 0xffff
    }
}

impl From<usize> for Status {
    fn from(value: usize) -> Self {
        let raw = RawStatus(value as u64);
        if raw.is_continued() {
            return Status::Continued;
        }

        if raw.is_stopped() {
            return Status::Stopped(raw.stop_sig());
        }

        if raw.is_signaled() {
            return Status::Signaled(raw.term_sig());
        }

        if raw.is_exited() {
            return Status::Exited(raw.exit_status());
        }

        Status::Invalid(value as u64)
    }
}

impl From<u64> for Status {
    fn from(value: u64) -> Self {
        Status::from(value as usize)
    }
}
impl From<u32> for Status {
    fn from(value: u32) -> Self {
        Status::from(value as usize)
    }
}

impl From<Status> for usize {
    fn from(value: Status) -> Self {
        match value {
            Status::Exited(v) => ((v as usize) & 0xff) << 8,
            Status::Signaled(v) => (v as usize) & 0x7f,
            Status::Continued => 0xffff,
            Status::Stopped(sig) => 0x7f | ((sig as usize) << 8),
            Status::Invalid(v) => v as usize,
        }
    }
}

impl From<Status> for u64 {
    fn from(value: Status) -> Self {
        usize::from(value) as Self
    }
}

impl From<Status> for u32 {
    fn from(value: Status) -> Self {
        usize::from(value) as Self
    }
}
