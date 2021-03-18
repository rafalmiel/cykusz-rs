pub const SIG_INT: usize = 0;

#[derive(Debug, Copy, Clone)]
pub enum SignalHandler {
    Ignore,
    Default,
    Handle(fn(usize)),
}

bitflags! {
    #[derive(Default)]
    pub struct SignalFlags: u64 {
        const RESTART = 1;
    }
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
