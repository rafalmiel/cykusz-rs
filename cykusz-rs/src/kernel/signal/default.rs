use alloc::sync::Arc;

use crate::kernel::task::Task;

#[derive(Copy, Clone, PartialEq)]
pub enum Action {
    Ignore,
    Handle(fn(usize)),
}

static DEFAULT_ACTIONS: [Action; super::SIGNAL_COUNT] = [
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate),        // SIGHUP
    Action::Handle(terminate),        // SIGINT
    Action::Handle(terminate),        // SIGQUIT
    Action::Handle(terminate),        // SIGILL
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate),        // SIGABRT
    Action::Handle(terminate),        // SIGBUS
    Action::Handle(terminate),        // SIGFPE
    Action::Handle(terminate),        // SIGKILL
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate),        // SIGSEGV
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate),        // SIGPIPE
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate),        // SIGTERM
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // SIGCHLD
    Action::Ignore,                   // SIGCONT
    Action::Handle(stop),             // SIGSTOP
    Action::Handle(stop),             // SIGTSTP
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Ignore,                   // UNUSED
    Action::Handle(terminate_thread), // UNUSED
];

fn terminate(sig: usize) {
    crate::kernel::sched::exit(sig.into());
}

fn terminate_thread(_sig: usize) {
    crate::kernel::sched::exit_thread()
}

fn stop(sig: usize) {
    crate::kernel::sched::stop(sig);
}

pub fn cont(sig: usize, task: Arc<Task>) {
    logln2!("CONT {}", task.tid());
    crate::kernel::sched::cont(task);
}

pub(in crate::kernel::signal) fn ignore_by_default(sig: usize) -> bool {
    DEFAULT_ACTIONS[sig] == Action::Ignore
}

pub(in crate::kernel::signal) fn action(sig: usize) -> Action {
    DEFAULT_ACTIONS[sig]
}

pub(in crate::kernel::signal) fn handle_default(sig: usize) {
    let action = DEFAULT_ACTIONS[sig];

    if let Action::Handle(f) = action {
        (f)(sig);
    }
}
