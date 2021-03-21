#[derive(Copy, Clone)]
enum Action {
    Ignore,
    Handle(fn()),
}

static DEFAULT_ACTIONS: [Action; super::SIGNAL_COUNT] = [
    Action::Ignore,            // UNUSED
    Action::Ignore,            // UNUSED
    Action::Handle(terminate), // SIG_INT
];

fn terminate() {
    crate::kernel::sched::task_finished();
}

pub(in crate::kernel::signal) fn handle_default(sig: usize) {
    let action = DEFAULT_ACTIONS[sig];

    if let Action::Handle(f) = action {
        (f)();
    }
}
