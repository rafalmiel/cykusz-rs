#![allow(dead_code)]

use bit_field::BitField;
use core::sync::atomic::{AtomicU64, Ordering};
use syscall_defs::SyscallError;

use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;

mod default;

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
    flags: syscall_defs::signal::SignalFlags,
    sigreturn: usize,
}

impl SignalEntry {
    pub fn handler(&self) -> syscall_defs::signal::SignalHandler {
        self.handler
    }

    pub fn flags(&self) -> syscall_defs::signal::SignalFlags {
        self.flags
    }

    pub fn sigreturn(&self) -> usize {
        self.sigreturn
    }
}

const SIGNAL_COUNT: usize = 3;

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

    pub fn trigger(&self, signal: usize) {
        assert!(signal < SIGNAL_COUNT);

        self.pending_mask.fetch_or(1u64 << signal, Ordering::SeqCst);
    }

    pub fn set_signal(
        &self,
        signal: usize,
        handler: syscall_defs::signal::SignalHandler,
        flags: syscall_defs::signal::SignalFlags,
        sigreturn: usize,
    ) {
        assert!(signal < SIGNAL_COUNT);

        let mut signals = self.entries.lock();

        println!("add handler {:?}", handler);

        signals[signal] = SignalEntry {
            handler,
            flags,
            sigreturn,
        };
    }

    pub fn copy_from(&self, signals: &Signals) {
        *self.entries.lock() = *signals.entries.lock();

        self.pending_mask.store(
            signals.pending_mask.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
        self.blocked_mask.store(
            signals.blocked_mask.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
    }
}

pub fn do_signals() -> Option<(usize, SignalEntry)> {
    let mut task = current_task();

    let mut signals = task.signals();

    if !signals.has_pending() {
        return None;
    }

    let pending = signals.pending();

    for s in 0..SIGNAL_COUNT {
        if pending.get_bit(s) {
            let entry = signals.entries.lock()[s];

            signals.pending_mask.store(0, Ordering::SeqCst);

            match entry.handler() {
                syscall_defs::signal::SignalHandler::Default => {
                    drop(signals);

                    task = default::handle_default(s, task);
                    signals = task.signals();
                }
                syscall_defs::signal::SignalHandler::Handle(_) => {
                    return Some((s, entry));
                }
                syscall_defs::signal::SignalHandler::Ignore => {}
            }
        }
    }

    None
}
