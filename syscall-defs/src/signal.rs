pub const SIGHUP: usize = 1;
pub const SIGINT: usize = 2;
pub const SIGQUIT: usize = 3;
pub const SIGILL: usize = 4;
pub const SIGABRT: usize = 6;
pub const SIGBUS: usize = 7;
pub const SIGFPE: usize = 8;
pub const SIGKILL: usize = 9;
pub const SIGSEGV: usize = 11;
pub const SIGPIPE: usize = 13;
pub const SIGTERM: usize = 15;
pub const SIGCHLD: usize = 17;
pub const SIGCONT: usize = 18;
pub const SIGSTOP: usize = 19;
pub const SIGTSTP: usize = 20;
pub const SIGTTIN: usize = 21;
pub const SIGTTOU: usize = 22;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SignalHandler {
    Ignore,
    Default,
    Handle(fn(usize)),
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SigAction {
    pub sa_handler: u64,
    pub sa_flags: u32,
    pub sa_restorer: u64,
    pub sa_mask: u64,
}

impl SigAction {
    pub fn new(handler: SignalHandler, mask: u64, flags: SignalFlags) -> SigAction {
        SigAction {
            sa_handler: handler.into(),
            sa_flags: flags.bits(),
            sa_restorer: 0,
            sa_mask: mask,
        }
    }
}

bitflags! {
    #[derive(Default, Copy, Clone, Debug)]
    pub struct SignalFlags: u32 {
        const RESTART = 0x10000000;
    }
}

#[repr(u64)]
#[derive(Debug)]
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
        let v = v as i64;
        match v {
            1 => SignalHandler::Ignore,
            0 => SignalHandler::Default,
            v => SignalHandler::Handle(unsafe { core::mem::transmute::<u64, fn(usize)>(v as u64) }),
        }
    }
}

impl From<SignalHandler> for usize {
    fn from(h: SignalHandler) -> Self {
        match h {
            SignalHandler::Ignore => 1isize as usize,
            SignalHandler::Default => 0isize as usize,
            SignalHandler::Handle(f) => f as usize,
        }
    }
}

impl From<SignalHandler> for u64 {
    fn from(h: SignalHandler) -> Self {
        match h {
            SignalHandler::Ignore => 1isize as u64,
            SignalHandler::Default => 0isize as u64,
            SignalHandler::Handle(f) => f as u64,
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
