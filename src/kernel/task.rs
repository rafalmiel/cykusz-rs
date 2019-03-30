use arch::task::Task as ArchTask;
use kernel::mm::MappedAddr;
use kernel::sched::new_task_id;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Running = 1,
    Runnable = 2,
    ToReschedule = 3,
    ToDelete = 4,
}

pub struct Task {
    pub arch_task: ArchTask,
    pub id: usize,
    pub state: TaskState,
    pub locks: i32,
}

impl Task {
    pub fn this() -> Task {
        Task {
            arch_task: ArchTask::empty(),
            id: new_task_id(),
            state: TaskState::Running,
            locks: 0,
        }
    }

    pub fn new_sched(fun: fn()) -> Task {
        Task {
            arch_task: ArchTask::new_sched(fun),
            id: new_task_id(),
            state: TaskState::Runnable,
            locks: 0
        }
    }

    pub fn new_kern(fun: fn()) -> Task {
        Task {
            arch_task: ArchTask::new_kern(fun),
            id: ::kernel::sched::new_task_id(),
            state: TaskState::Runnable,
            locks: 0,
        }
    }

    pub fn new_user(fun: MappedAddr, code_size: usize, stack: usize) -> Task {
        Task {
            arch_task: ArchTask::new_user(fun, code_size, stack),
            id: ::kernel::sched::new_task_id(),
            state: TaskState::Runnable,
            locks: 0,
        }
    }

    pub fn deallocate(&mut self) {
        self.arch_task.deallocate();
        self.state = TaskState::ToDelete;
    }

}