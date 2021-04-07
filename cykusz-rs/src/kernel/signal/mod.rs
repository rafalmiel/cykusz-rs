#![allow(dead_code)]

use alloc::sync::Arc;
use core::ops::{Index, IndexMut};
use core::sync::atomic::{AtomicU64, Ordering};

use bit_field::BitField;

use syscall_defs::signal::SignalFlags;
use syscall_defs::signal::SignalHandler;
use syscall_defs::SyscallError;

use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::{Spin, SpinGuard};

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
            SignalError::Interrupted => SyscallError::EINTR,
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
pub struct Entries {
    entries: [SignalEntry; SIGNAL_COUNT],
    pending_mask: u64,
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

impl Entries {
    pub fn has_pending(&self) -> bool {
        self.pending() > 0
    }

    pub fn pending(&self) -> u64 {
        self.pending_mask
    }

    pub fn is_pending(&self, sig: u64) -> bool {
        self.pending_mask.get_bit(sig as usize)
    }

    pub fn clear_pending(&mut self, sig: u64) {
        self.pending_mask.set_bit(sig as usize, false);
    }

    pub fn set_pending(&mut self, sig: u64) {
        self.pending_mask.set_bit(sig as usize, true);
    }
}

#[derive(Default)]
pub struct Signals {
    entries: Arc<Spin<Entries>>,
    blocked_mask: AtomicU64,
}

impl Clone for Signals {
    fn clone(&self) -> Self {
        Signals {
            entries: self.entries.clone(),
            blocked_mask: AtomicU64::new(self.blocked_mask.load(Ordering::SeqCst)),
        }
    }
}

pub enum TriggerResult {
    Ignored,
    Blocked,
    Triggered,
}

impl Signals {
    pub fn entries(&self) -> SpinGuard<Entries> {
        self.entries.lock_irq()
    }

    pub fn has_pending(&self) -> bool {
        self.entries().pending() & !self.blocked_mask() > 0
    }

    pub fn blocked_mask(&self) -> u64 {
        self.blocked_mask.load(Ordering::SeqCst)
    }

    pub fn is_blocked(&self, signal: usize) -> bool {
        self.blocked_mask().get_bit(signal)
    }

    pub fn trigger(&self, signal: usize) -> TriggerResult {
        assert!(signal < SIGNAL_COUNT);

        let mut sigs = self.entries();

        let handler = sigs[signal].handler();

        if match handler {
            SignalHandler::Ignore => false,
            SignalHandler::Default => !default::ignore_by_default(signal),
            SignalHandler::Handle(_) => true,
        } {
            sigs.set_pending(signal as u64);

            if self.is_blocked(signal) {
                TriggerResult::Blocked
            } else {
                TriggerResult::Triggered
            }
        } else {
            TriggerResult::Ignored
        }
    }

    pub fn clear(&self) {
        *self.entries.lock_irq() = Entries::default();
        self.blocked_mask.store(0, Ordering::SeqCst);
    }

    pub fn set_signal(
        &self,
        signal: usize,
        handler: SignalHandler,
        flags: SignalFlags,
        sigreturn: usize,
    ) {
        assert!(signal < SIGNAL_COUNT);

        let mut signals = self.entries();

        signals[signal] = SignalEntry {
            handler,
            flags,
            sigreturn,
        };
    }

    pub fn copy_from(&self, signals: &Signals) {
        *self.entries() = *signals.entries();

        self.blocked_mask.store(
            signals.blocked_mask.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
    }

    pub fn set_mask(
        &self,
        how: syscall_defs::signal::SigProcMask,
        set: u64,
        old_set: Option<&mut u64>,
    ) {
        if let Some(old) = old_set {
            *old = self.blocked_mask.load(Ordering::SeqCst);
        }

        match how {
            syscall_defs::signal::SigProcMask::Block => {
                self.blocked_mask.fetch_or(set, Ordering::SeqCst);
            }
            syscall_defs::signal::SigProcMask::Unblock => {
                self.blocked_mask.fetch_and(!set, Ordering::SeqCst);
            }
            syscall_defs::signal::SigProcMask::Set => {
                self.blocked_mask.store(set, Ordering::SeqCst);
            }
        }
    }
}

pub fn do_signals() -> Option<(usize, SignalEntry)> {
    let task = current_task_ref();

    if task.is_terminate_thread() {
        crate::kernel::sched::exit_thread();
    }

    let signals = task.signals();

    if !signals.has_pending() {
        return None;
    }

    let mut entries = signals.entries();

    for s in 0..SIGNAL_COUNT {
        if !signals.is_blocked(s) && entries.is_pending(s as u64) {
            let entry = entries[s];

            entries.clear_pending(s as u64);

            match entry.handler() {
                SignalHandler::Default => {
                    drop(entries);
                    default::handle_default(s);
                    entries = task.signals().entries();
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
