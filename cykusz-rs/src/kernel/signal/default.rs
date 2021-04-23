use crate::kernel::task::Task;

use alloc::sync::Arc;

#[derive(Copy, Clone, PartialEq)]
pub enum Action {
    Ignore,
    Handle(fn()),
    Exec(fn(Arc<Task>)),
}

static DEFAULT_ACTIONS: [Action; super::SIGNAL_COUNT] = [
    Action::Ignore,            // UNUSED
    Action::Handle(terminate), // SIGHUP
    Action::Handle(terminate), // SIGINT
    Action::Handle(terminate), // SIGQUIT
    Action::Handle(terminate), // SIGILL
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Handle(terminate), // SIGBUS
    Action::Handle(terminate), // SIGFPE
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Handle(terminate), // SIGSEGV
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Ignore,            // SIGCHLD
    Action::Exec(cont),        // SIGCONT
    Action::Handle(stop),      // SIGSTOP
];

fn terminate() {
    crate::kernel::sched::exit();
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
