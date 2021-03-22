#![allow(dead_code)]

use bit_field::BitField;
use core::ops::{Index, IndexMut};
use core::sync::atomic::{AtomicU64, Ordering};
use syscall_defs::signal::SignalFlags;
use syscall_defs::signal::SignalHandler;
use syscall_defs::SyscallError;

use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task_ref;
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
    handler: SignalHandler,
    flags: SignalFlags,
    sigreturn: usize,
}

impl SignalEntry {
    pub fn ignore() -> SignalEntry {
        SignalEntry {
            handler: SignalHandler::Ignore,
            flags: SignalFlags::empty(),
            sigreturn: 0,
        }
    }

    pub fn handler(&self) -> SignalHandler {
        self.handler
    }

    pub fn flags(&self) -> SignalFlags {
        self.flags
    }

    pub fn sigreturn(&self) -> usize {
        self.sigreturn
    }
}

const SIGNAL_COUNT: usize = 18;

#[derive(Copy, Clone, Default)]
struct Entries {
    entries: [SignalEntry; SIGNAL_COUNT],
}

impl Index<usize> for Entries {
    type Output = SignalEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for Entries {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

#[derive(Default)]
pub struct Signals {
    entries: Spin<Entries>,
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

    pub fn trigger(&self, signal: usize) -> bool {
        assert!(signal < SIGNAL_COUNT);

        let sigs = self.entries.lock();

        let handler = sigs[signal].handler();

        if match handler {
            SignalHandler::Ignore => false,
            SignalHandler::Default => !default::ignore_by_default(signal),
            SignalHandler::Handle(_) => true,
        } {
            self.pending_mask.fetch_or(1u64 << signal, Ordering::SeqCst);

            true
        } else {
            false
        }
    }

    pub fn clear(&self) {
        *self.entries.lock() = Entries::default();

        self.pending_mask.store(0, Ordering::SeqCst);
    }

    pub fn set_signal(
        &self,
        signal: usize,
        handler: SignalHandler,
        flags: SignalFlags,
        sigreturn: usize,
    ) {
        assert!(signal < SIGNAL_COUNT);

        let mut signals = self.entries.lock();

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
    let task = current_task_ref();

    let signals = task.signals();

    if !signals.has_pending() {
        return None;
    }

    let pending = signals.pending();

    for s in 0..SIGNAL_COUNT {
        if pending.get_bit(s) {
            let entry = signals.entries.lock()[s];

            signals.pending_mask.store(0, Ordering::SeqCst);

            match entry.handler() {
                SignalHandler::Default => {
                    default::handle_default(s);
                }
                SignalHandler::Handle(_) => {
                    return Some((s, entry));
                }
                SignalHandler::Ignore => {
                    unreachable!()
                }
            }
        }
    }

    None
}
