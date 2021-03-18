use alloc::sync::Arc;

use crate::kernel::task::Task;

#[derive(Copy, Clone)]
enum Action {
    Ignore,
    Handle(fn(Arc<Task>) -> Arc<Task>),
}

static DEFAULT_ACTIONS: [Action; super::SIGNAL_COUNT] = [
    Action::Handle(terminate), // SIG_INT
];

fn terminate(task: Arc<Task>) -> Arc<Task> {
    drop(task);
    crate::kernel::sched::task_finished();
}

pub(in crate::kernel::signal) fn handle_default(sig: usize, mut task: Arc<Task>) -> Arc<Task> {
    let action = DEFAULT_ACTIONS[sig];

    if let Action::Handle(f) = action {
        task = (f)(task);
    }

    task
}
