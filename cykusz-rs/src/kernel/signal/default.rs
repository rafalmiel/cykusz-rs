use alloc::sync::Arc;
use crate::kernel::sched::current_task;

use crate::kernel::task::Task;

#[derive(Copy, Clone, PartialEq)]
pub enum Action {
    Ignore,
    Handle(fn()),
    Exec(fn(Arc<Task>)),
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
    Action::Exec(cont),               // SIGCONT
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

fn terminate() {
    crate::kernel::sched::exit(1);
}

fn terminate_thread() {
    crate::kernel::sched::exit_thread()
}

fn stop() {
    crate::kernel::sched::stop();
}

fn cont(task: Arc<Task>) {
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
        (f)();
    }
}
