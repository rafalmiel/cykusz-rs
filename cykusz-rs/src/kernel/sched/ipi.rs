use crate::kernel::ipi;
use crate::kernel::task::{ArcTask, Task};

fn handle_ipi_queue(args: ipi::r#async::AsyncIpiArg) {
    let task = unsafe { ArcTask::from_raw(args as *const Task) };

    crate::kernel::sched::internal().queue_task(task, false);
}

pub fn send_ipi_queue(task: &ArcTask) {
    ipi::r#async::call_async(
        ipi::IpiTarget::Cpu(task.on_cpu()),
        handle_ipi_queue,
        ArcTask::into_raw(task.clone()) as *mut (),
    );
}

fn handle_ipi_cont(args: ipi::r#async::AsyncIpiArg) {
    let task = unsafe { ArcTask::from_raw(args as *const Task) };

    crate::kernel::sched::internal().cont(task);
}

pub fn send_ipi_cont(task: &ArcTask) {
    ipi::r#async::call_async(
        ipi::IpiTarget::Cpu(task.on_cpu()),
        handle_ipi_cont,
        ArcTask::into_raw(task.clone()) as *mut (),
    );
}

fn handle_ipi_wake_up(args: ipi::r#async::AsyncIpiArg) {
    let task = unsafe { ArcTask::from_raw(args as *const Task) };

    crate::kernel::sched::internal().wake(task);
}

pub fn send_ipi_wake_up(task: &ArcTask) {
    ipi::r#async::call_async(
        ipi::IpiTarget::Cpu(task.on_cpu()),
        handle_ipi_wake_up,
        ArcTask::into_raw(task.clone()) as *mut (),
    );
}

fn handle_ipi_wake_up_next(args: ipi::r#async::AsyncIpiArg) {
    let task = unsafe { ArcTask::from_raw(args as *const Task) };

    crate::kernel::sched::internal().wake_as_next(task);
}

pub fn send_ipi_wake_up_next(task: &ArcTask) {
    ipi::r#async::call_async(
        ipi::IpiTarget::Cpu(task.on_cpu()),
        handle_ipi_wake_up_next,
        ArcTask::into_raw(task.clone()) as *mut (),
    );
}

/// Conditionally do ipi if task is on another cpu
pub fn maybe_do_ipi(task: &ArcTask, fun: fn(&ArcTask)) -> bool {
    if task.is_on_this_cpu() {
        return false;
    }

    fun(task);
    true
}
