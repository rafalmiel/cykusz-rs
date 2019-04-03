use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::task::Task as ArchTask;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sched::new_task_id;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Running = 1,
    Runnable = 2,
    ToReschedule = 3,
    ToDelete = 4,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            1 => Running,
            2 => Runnable,
            3 => ToReschedule,
            4 => ToDelete,
            _ => unreachable!(),
        }
    }
}

pub struct Task {
    pub arch_task: UnsafeCell<ArchTask>,
    id: usize,
    state: AtomicUsize,
    locks: AtomicUsize,
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::empty()),
            id: new_task_id(),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
        }
    }

    pub fn new_sched(fun: fn()) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::new_sched(fun)),
            id: new_task_id(),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
        }
    }

    pub fn new_kern(fun: fn()) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::new_kern(fun)),
            id: crate::kernel::sched::new_task_id(),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
        }
    }

    pub fn new_user(fun: MappedAddr, code_size: usize, stack: usize) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::new_user(fun, code_size, stack)),
            id: crate::kernel::sched::new_task_id(),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
        }
    }

    pub fn set_state(&self, state: TaskState) {
        self.state.store(state as usize, Ordering::SeqCst);
    }

    pub fn state(&self) -> TaskState {
        self.state.load(Ordering::SeqCst).into()
    }

    pub fn locks_inc(&self) {
        self.locks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn locks_dec(&self) {
        self.locks.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn locks(&self) -> usize {
        self.locks.load(Ordering::SeqCst)
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub unsafe fn arch_task_mut(&self) -> &mut ArchTask {
        &mut (*self.arch_task.get())
    }

    pub unsafe fn arch_task(&self) -> &ArchTask {
        &(*self.arch_task.get())
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            self.arch_task_mut().deallocate();
        }
    }
}
