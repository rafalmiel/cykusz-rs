#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};
use syscall_defs::SyscallError;

use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use syscall_defs::signal::SIG_INT;

#[derive(Debug, PartialEq)]
pub enum SignalError {
    Interrupted,
}

pub type SignalResult<T> = core::result::Result<T, SignalError>;

impl From<SignalError> for FsError {
    fn from(s: SignalError) -> Self {
        match s {
            SignalError::Interrupted => FsError::Interrupted,
        }
    }
}

impl From<SignalError> for SyscallError {
    fn from(s: SignalError) -> Self {
        match s {
            SignalError::Interrupted => SyscallError::Interrupted,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct SignalEntry {
    handler: syscall_defs::signal::SignalHandler,
    sigreturn: usize,
}

impl SignalEntry {
    pub fn handler(&self) -> syscall_defs::signal::SignalHandler {
        self.handler
    }

    pub fn sigreturn(&self) -> usize {
        self.sigreturn
    }
}

const SIGNAL_COUNT: usize = 1;

#[derive(Default)]
pub struct Signals {
    entries: Spin<[SignalEntry; SIGNAL_COUNT]>,
    blocked_mask: AtomicU64,
    pending_mask: AtomicU64,
}

impl Signals {
    pub fn has_pending(&self) -> bool {
        self.pending() > 0
    }

    pub fn pending(&self) -> u64 {
        self.pending_mask.load(Ordering::SeqCst)
    }

    pub fn do_signals(&self) -> Option<SignalEntry> {
        let pending = self.pending();

        self.pending_mask.store(0, Ordering::SeqCst);

        if pending & (1u64 << SIG_INT) > 0 {
            Some(self.entries.lock()[SIG_INT])
        } else {
            None
        }
    }

    pub fn trigger(&self, signal: usize) {
        assert!(signal < SIGNAL_COUNT);

        self.pending_mask.fetch_or(1u64 << signal, Ordering::SeqCst);
    }

    pub fn set_signal(
        &self,
        signal: usize,
        handler: syscall_defs::signal::SignalHandler,
        sigreturn: usize,
    ) {
        assert!(signal < SIGNAL_COUNT);

        let mut signals = self.entries.lock();

        signals[signal] = SignalEntry { handler, sigreturn };
    }
}

pub fn do_signals() {
    current_task().signals().do_signals();
}
