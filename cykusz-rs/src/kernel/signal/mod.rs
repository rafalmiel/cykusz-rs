#![allow(dead_code)]

use alloc::sync::Arc;
use core::any::Any;
use core::ops::{Index, IndexMut};
use core::sync::atomic::{AtomicU64, Ordering};

use bit_field::BitField;

use syscall_defs::signal::SignalHandler;
use syscall_defs::signal::{SigAction, SignalFlags};
use syscall_defs::{SyscallError, SyscallResult};

use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::{IrqGuard, LockApi, Spin, SpinGuard};
use crate::kernel::task::Task;

mod default;

pub const KSIGKILLTHR: usize = 32;
pub const KSIGSTOPTHR: usize = 33;
pub const KSIGEXEC: usize = 63;

#[derive(Debug, PartialEq)]
pub enum SignalError {
    Interrupted,
}

const IMMUTABLE_MASK: u64 = {
    let a = (1u64 << syscall_defs::signal::SIGSTOP - 1)
        | (1u64 << syscall_defs::signal::SIGABRT - 1)
        | (1u64 << syscall_defs::signal::SIGKILL - 1)
        | (1u64 << KSIGKILLTHR - 1)
        | (1u64 << KSIGSTOPTHR - 1)
        | (1u64 << KSIGEXEC - 1);

    a
};

const UNBLOCKABLE_MASK: u64 = {
    let a = (1u64 << syscall_defs::signal::SIGSTOP - 1)
        | (1u64 << syscall_defs::signal::SIGCONT - 1)
        | (1u64 << syscall_defs::signal::SIGABRT - 1)
        | (1u64 << syscall_defs::signal::SIGKILL - 1)
        | (1u64 << KSIGKILLTHR - 1)
        | (1u64 << KSIGSTOPTHR - 1)
        | (1u64 << KSIGEXEC - 1);

    a
};

fn can_override(sig: usize) -> bool {
    IMMUTABLE_MASK.get_bit(sig - 1) == false
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
    mask: u64,
    sigreturn: usize,
}

impl SignalEntry {
    pub fn from_sigaction(act: SigAction) -> core::result::Result<SignalEntry, SyscallError> {
        Ok(SignalEntry {
            handler: SignalHandler::from(act.sa_handler),
            flags: SignalFlags::from_bits(act.sa_flags).ok_or(SyscallError::EINVAL)?,
            mask: act.sa_mask,
            sigreturn: act.sa_restorer as usize,
        })
    }

    pub fn to_sigaction(&self) -> SigAction {
        let h: usize = self.handler.into();
        SigAction {
            sa_handler: h as u64,
            sa_mask: self.mask,
            sa_flags: self.flags.bits(),
            sa_restorer: self.sigreturn as u64,
        }
    }
}

impl SignalEntry {
    pub fn ignore() -> SignalEntry {
        SignalEntry {
            handler: SignalHandler::Ignore,
            flags: SignalFlags::empty(),
            mask: 0,
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

const SIGNAL_COUNT: usize = 34;

#[derive(Copy, Clone)]
pub struct Entries {
    entries: [SignalEntry; SIGNAL_COUNT],
    pending_mask: u64,
}

impl Default for Entries {
    fn default() -> Entries {
        Entries {
            entries: [SignalEntry::default(); SIGNAL_COUNT],
            pending_mask: 0,
        }
    }
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
        let sig = sig - 1;
        self.pending_mask.get_bit(sig as usize)
    }

    pub fn clear_pending(&mut self, sig: u64) {
        let sig = sig - 1;
        self.pending_mask.set_bit(sig as usize, false);
    }

    pub fn set_pending(&mut self, sig: u64) {
        let sig = sig - 1;
        self.pending_mask.set_bit(sig as usize, true);
    }
}

pub type SigExecParam = Arc<dyn Any + Send + Sync>;

pub struct SigExec {
    handler: fn(SigExecParam),
    param: SigExecParam,
}

#[derive(Default)]
pub struct Signals {
    entries: Arc<Spin<Entries>>,
    blocked_mask: AtomicU64,
    thread_pending_mask: AtomicU64,

    sig_exec: Spin<Option<SigExec>>,
}

impl Clone for Signals {
    fn clone(&self) -> Self {
        Signals {
            entries: self.entries.clone(),
            blocked_mask: AtomicU64::new(self.blocked_mask.load(Ordering::SeqCst)),
            thread_pending_mask: AtomicU64::new(0),

            sig_exec: Spin::new(None),
        }
    }
}

#[derive(Debug)]
pub enum TriggerResult {
    Blocked,
    Triggered,
    Execute(fn(usize, Arc<Task>)),
}

impl Signals {
    pub fn entries(&self) -> SpinGuard<'_, Entries> {
        self.entries.lock_irq()
    }

    pub fn thread_pending(&self) -> u64 {
        self.thread_pending_mask.load(Ordering::SeqCst)
    }

    pub fn pending(&self) -> u64 {
        self.thread_pending() | self.entries().pending()
    }

    pub fn is_pending(&self, sig: u64) -> bool {
        logln2!(
            "is blocked: {} {}",
            sig,
            self.pending().get_bit(sig as usize - 1)
        );
        let sig = sig - 1;
        self.pending().get_bit(sig as usize)
    }

    pub fn clear_pending(&self, sig: u64) {
        let sig = sig - 1;
        if self.thread_pending().get_bit(sig as usize) {
            self.thread_pending_mask
                .fetch_and(!(1u64 << sig), Ordering::SeqCst);
        } else {
            self.entries().clear_pending(sig + 1);
        }
    }

    pub fn set_pending(&self, sig: u64, thread_scope: bool) {
        let sig = sig - 1;
        if thread_scope {
            self.thread_pending_mask
                .fetch_or(1u64 << sig, Ordering::SeqCst);
        } else {
            self.entries().set_pending(sig + 1);
        }
    }

    pub fn has_pending(&self) -> bool {
        ((self.entries().pending() | self.thread_pending()) & !self.blocked_mask()) > 0
    }

    pub fn blocked_mask(&self) -> u64 {
        self.blocked_mask.load(Ordering::SeqCst)
    }

    pub fn is_blocked(&self, signal: usize) -> bool {
        logln2!(
            "is blocked: {} {}",
            signal,
            self.blocked_mask().get_bit(signal - 1)
        );
        let signal = signal - 1;
        self.blocked_mask().get_bit(signal)
    }

    pub fn setup_sig_exec(&self, f: fn(SigExecParam), param: SigExecParam) -> bool {
        if !self.is_pending(KSIGEXEC as u64) {
            *self.sig_exec.lock() = Some(SigExec { handler: f, param });

            self.set_pending(KSIGEXEC as u64, true);

            true
        } else {
            false
        }
    }

    pub fn sig_exec(&self) {
        if self.is_pending(KSIGEXEC as u64) {
            self.clear_pending(KSIGEXEC as u64);

            let exc = self.sig_exec.lock().take();

            if let Some(SigExec { handler, param }) = exc {
                let _ = IrqGuard::new();
                (handler)(param);
            }
        }
    }

    pub fn trigger(&self, signal: usize, this_thread: bool) -> TriggerResult {
        assert!(signal < SIGNAL_COUNT);

        let set_pending = |trigger_result: TriggerResult| {
            self.set_pending(signal as u64, this_thread);

            if self.is_blocked(signal) {
                TriggerResult::Blocked
            } else {
                trigger_result
            }
        };

        if signal == syscall_defs::signal::SIGCONT || signal == syscall_defs::signal::SIGKILL {
            set_pending(TriggerResult::Execute(default::cont))
        } else {
            set_pending(TriggerResult::Triggered)
        }
    }

    pub fn clear(&self) {
        *self.entries.lock_irq() = Entries::default();
        self.blocked_mask.store(0, Ordering::SeqCst);
    }

    pub fn set_signal(
        &self,
        signal: usize,
        handler: Option<SignalEntry>,
        old: Option<&mut syscall_defs::signal::SigAction>,
    ) -> SyscallResult {
        if signal >= SIGNAL_COUNT {
            return Err(SyscallError::EINVAL);
        }

        if !can_override(signal) {
            return Err(SyscallError::EINVAL);
        }

        let mut signals = self.entries();

        if let Some(old) = old {
            *old = signals[signal].to_sigaction();
        }

        if let Some(handler) = handler {
            signals[signal] = handler;
        }

        Ok(0)
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
        set: Option<u64>,
        old_set: Option<&mut u64>,
    ) {
        if let Some(old) = old_set {
            *old = self.blocked_mask.load(Ordering::SeqCst);
        }

        if set.is_none() {
            return;
        }

        let set = set.unwrap();

        let set = set & !UNBLOCKABLE_MASK;

        match how {
            syscall_defs::signal::SigProcMask::Block => {
                self.blocked_mask.fetch_or(set, Ordering::SeqCst);
                logln2!(
                    "sigblock {:b} {:b}",
                    set,
                    self.blocked_mask.load(Ordering::SeqCst)
                );
            }
            syscall_defs::signal::SigProcMask::Unblock => {
                self.blocked_mask.fetch_and(!set, Ordering::SeqCst);
                logln2!(
                    "sigunblock {:b} {:b}",
                    set,
                    self.blocked_mask.load(Ordering::SeqCst)
                );
            }
            syscall_defs::signal::SigProcMask::Set => {
                self.blocked_mask.store(set, Ordering::SeqCst);
                logln2!(
                    "sigset {:b} {:b}",
                    set,
                    self.blocked_mask.load(Ordering::SeqCst)
                );
            }
        }
    }
}

pub enum DoSignalsResult {
    None,
    Entry((usize, SignalEntry)),
    Default,
    Ignore,
}

pub fn do_signals(_from_syscall: bool, _syscall: usize) -> DoSignalsResult {
    let task = current_task_ref();

    let signals = task.signals();

    if !signals.has_pending() {
        return DoSignalsResult::None;
    }

    if signals.is_pending(syscall_defs::signal::SIGKILL as u64) {
        logln_disabled!(
            "sigkill: {} sc: {}, wc: {}",
            task.tid(),
            Arc::strong_count(task),
            Arc::weak_count(task)
        );

        signals.clear_pending(syscall_defs::signal::SIGKILL as u64);

        crate::kernel::sched::exit(syscall_defs::waitpid::Status::Signaled(
            syscall_defs::signal::SIGKILL as u64,
        ));
    }
    if signals.is_pending(syscall_defs::signal::SIGABRT as u64) {
        logln_disabled!(
            "sigkill: {} sc: {}, wc: {}",
            task.tid(),
            Arc::strong_count(task),
            Arc::weak_count(task)
        );

        signals.clear_pending(syscall_defs::signal::SIGABRT as u64);

        crate::kernel::sched::exit(syscall_defs::waitpid::Status::Signaled(
            syscall_defs::signal::SIGABRT as u64,
        ));
    }

    signals.sig_exec();

    for s in 1..SIGNAL_COUNT {
        if !signals.is_blocked(s) && signals.is_pending(s as u64) {
            signals.clear_pending(s as u64);

            let entries = signals.entries();

            let entry = entries[s];

            return match entry.handler() {
                SignalHandler::Default => {
                    drop(entries);
                    default::handle_default(s);
                    DoSignalsResult::Default
                }
                SignalHandler::Handle(_) => DoSignalsResult::Entry((s, entry)),
                SignalHandler::Ignore => DoSignalsResult::Ignore,
            };
        }
    }

    DoSignalsResult::None
}
