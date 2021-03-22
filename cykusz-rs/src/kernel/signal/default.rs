#[derive(Copy, Clone, PartialEq)]
enum Action {
    Ignore,
    Handle(fn()),
}

static DEFAULT_ACTIONS: [Action; super::SIGNAL_COUNT] = [
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
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
];

fn terminate() {
    crate::kernel::sched::task_finished();
}

pub(in crate::kernel::signal) fn ignore_by_default(sig: usize) -> bool {
    DEFAULT_ACTIONS[sig] == Action::Ignore
}

pub(in crate::kernel::signal) fn handle_default(sig: usize) {
    let action = DEFAULT_ACTIONS[sig];

    if let Action::Handle(f) = action {
        (f)();
    }
}
