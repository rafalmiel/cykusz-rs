use alloc::string::String;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::OpenFlags;

use crate::arch::task::Task as ArchTask;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::root_inode;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sched::new_task_id;
use crate::kernel::sync::RwLock;
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;

pub mod cwd;
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
    cwd: RwLock<Cwd>,
}

impl Default for Task {
    fn default() -> Self {
        Task {
            arch_task: UnsafeCell::new(ArchTask::empty()),
            id: new_task_id(),
            prev_state: AtomicUsize::new(TaskState::Unused as usize),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            filetable: filetable::FileTable::new(),
            cwd: RwLock::new(Cwd::new("/", root_inode().self_inode())),
        }
    }
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Task {
        Task::default()
    }

    pub fn new_sched(fun: fn()) -> Task {
        let mut task = Task::default();
        task.arch_task = UnsafeCell::new(ArchTask::new_sched(fun));
        task
    }

    pub fn new_kern(fun: fn()) -> Task {
        let mut task = Task::default();
        task.arch_task = UnsafeCell::new(ArchTask::new_kern(fun));
        task
    }

    pub fn new_user(fun: MappedAddr, code_size: usize) -> Task {
        let mut task = Task::default();
        task.arch_task = UnsafeCell::new(ArchTask::new_user(fun, code_size));

        task
    }

    pub fn get_handle(&self, fd: usize) -> Option<FileHandle> {
        self.filetable.get_handle(fd)
    }

    pub fn get_cwd(&self) -> Option<Arc<dyn INode>> {
        Some(self.cwd.read().inode.clone())
    }

    pub fn get_pwd(&self) -> String {
        self.cwd.read().pwd.clone()
    }

    pub fn open_file(&self, inode: Arc<dyn INode>, flags: OpenFlags) -> Option<usize> {
        self.filetable.open_file(inode, flags)
    }

    pub fn close_file(&self, fd: usize) -> bool {
        self.filetable.close_file(fd)
    }

    pub fn set_cwd(&self, inode: Arc<dyn INode>, path: &str) {
        let mut cwd = self.cwd.write();

        cwd.inode = inode;
        cwd.apply_path(path);
    }

    pub fn set_state(&self, state: TaskState) {
        self.state.store(state as usize, Ordering::SeqCst);
    }

    pub fn mark_to_reschedule(&self) {
        if self.state.load(Ordering::SeqCst) != TaskState::ToReschedule as usize {
            self.prev_state
                .store(self.state.load(Ordering::SeqCst), Ordering::SeqCst);
            self.state
                .store(TaskState::ToReschedule as usize, Ordering::SeqCst);
        }
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
