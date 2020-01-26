use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::task::Task as ArchTask;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sched::new_task_id;
use crate::kernel::sync::Mutex;
use crate::kernel::task::filetable::FileHandle;

pub mod filetable;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Running = 1,
    Runnable = 2,
    ToReschedule = 3,
    AwaitingIo = 4,
    ToDelete = 5,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            1 => Running,
            2 => Runnable,
            3 => ToReschedule,
            4 => AwaitingIo,
            5 => ToDelete,
            _ => unreachable!(),
        }
    }
}

pub struct Task {
    pub arch_task: UnsafeCell<ArchTask>,
    id: usize,
    prev_state: AtomicUsize,
    state: AtomicUsize,
    locks: AtomicUsize,
    filetable: filetable::FileTable,
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::empty()),
            id: new_task_id(),
            prev_state: AtomicUsize::new(TaskState::Unused as usize),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            filetable: filetable::FileTable::new(),
        }
    }

    pub fn new_sched(fun: fn()) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::new_sched(fun)),
            id: new_task_id(),
            prev_state: AtomicUsize::new(TaskState::Unused as usize),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            filetable: filetable::FileTable::new(),
        }
    }

    pub fn new_kern(fun: fn()) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::new_kern(fun)),
            id: crate::kernel::sched::new_task_id(),
            prev_state: AtomicUsize::new(TaskState::Unused as usize),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            filetable: filetable::FileTable::new(),
        }
    }

    pub fn new_user(fun: MappedAddr, code_size: usize) -> Task {
        let mut task = Task {
            arch_task: UnsafeCell::new(ArchTask::new_user(fun, code_size)),
            id: crate::kernel::sched::new_task_id(),
            prev_state: AtomicUsize::new(TaskState::Unused as usize),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            filetable: filetable::FileTable::new(),
        };

        task.filetable.open_file(crate::kernel::fs::stdio::stdout().clone());
        task.filetable.open_file(crate::kernel::fs::stdio::stdin().clone());

        task
    }

    pub fn get_handle(&self, fd: usize) -> Option<FileHandle> {
        self.filetable.get_handle(fd)
    }

    pub fn set_state(&self, state: TaskState) {
        self.state.store(state as usize, Ordering::SeqCst);
    }

    pub fn mark_to_reschedule(&self) {
        self.prev_state
            .store(self.state.load(Ordering::SeqCst), Ordering::SeqCst);
        self.state
            .store(TaskState::ToReschedule as usize, Ordering::SeqCst);
    }

    pub fn unmark_to_reschedule(&self) {
        self.state
            .store(self.prev_state.load(Ordering::SeqCst), Ordering::SeqCst);
        self.prev_state
            .store(TaskState::Unused as usize, Ordering::SeqCst);
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
