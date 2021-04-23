pub const SIGHUP: usize = 1;
pub const SIGINT: usize = 2;
pub const SIGQUIT: usize = 3;
pub const SIGILL: usize = 4;
pub const SIGBUS: usize = 7;
pub const SIGFPE: usize = 8;
pub const SIGSEGV: usize = 11;
pub const SIGCHLD: usize = 17;
pub const SIGCONT: usize = 18;
pub const SIGSTOP: usize = 19;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SignalHandler {
    Ignore,
    Default,
    Handle(fn(usize)),
}

bitflags! {
    #[derive(Default)]
    pub struct SignalFlags: u64 {
        const RESTART = (1u64 << 3);
    }
}

#[repr(u64)]
pub enum SigProcMask {
    Block = 0,
    Unblock = 1,
    Set = 2,
}

impl Default for SignalHandler {
    fn default() -> Self {
        SignalHandler::Default
    }
}

impl From<u64> for SignalHandler {
    fn from(v: u64) -> Self {
        match v {
            0 => SignalHandler::Ignore,
            1 => SignalHandler::Default,
            v => SignalHandler::Handle(unsafe { core::mem::transmute::<u64, fn(usize)>(v) }),
        }
    }
}

impl From<SignalHandler> for usize {
    fn from(h: SignalHandler) -> Self {
        match h {
            SignalHandler::Ignore => 0,
            SignalHandler::Default => 1,
            SignalHandler::Handle(f) => f as usize,
        }
    }
}

impl From<u64> for SigProcMask {
    fn from(v: u64) -> Self {
        match v {
            0 => SigProcMask::Block,
            1 => SigProcMask::Unblock,
            2 => SigProcMask::Set,
            _ => panic!("Invalid SigProcMask {}", v),
        }
    }
}

impl From<SigProcMask> for usize {
    fn from(s: SigProcMask) -> Self {
        s as u64 as usize
    }
}
