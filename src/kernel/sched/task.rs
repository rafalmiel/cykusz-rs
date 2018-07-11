use crate::arch::task::Task as ArchTask;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Running = 1,
    Runnable = 2,
    ToReschedule = 3,
    ToDelete = 4,
}

#[derive(Copy, Clone, Debug)]
pub struct Task {
    pub arch_task: ArchTask,
    pub state: TaskState,
    pub locks: u32,
}