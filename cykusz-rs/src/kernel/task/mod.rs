use alloc::string::String;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use syscall_defs::OpenFlags;

use crate::arch::task::Task as ArchTask;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::root_dentry;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sched::new_task_id;
use crate::kernel::sync::RwSpin;
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;

pub mod cwd;
pub mod filetable;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Running = 1,
    Runnable = 2,
    AwaitingIo = 4,
    Halted = 5,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            1 => Running,
            2 => Runnable,
            4 => AwaitingIo,
            5 => Halted,
            _ => unreachable!(),
        }
    }
}

pub struct Task {
    pub arch_task: UnsafeCell<ArchTask>,
    id: usize,
    state: AtomicUsize,
    locks: AtomicUsize,
    pending_io: AtomicBool,
    to_resched: AtomicBool,
    to_delete: AtomicBool,
    halted: AtomicBool,
    filetable: filetable::FileTable,
    pub sleep_until: AtomicUsize,
    cwd: RwSpin<Cwd>,
}

impl Default for Task {
    fn default() -> Self {
        Task {
            arch_task: UnsafeCell::new(ArchTask::empty()),
            id: new_task_id(),
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            pending_io: AtomicBool::new(false),
            to_resched: AtomicBool::new(false),
            to_delete: AtomicBool::new(false),
            halted: AtomicBool::new(false),
            filetable: filetable::FileTable::new(),
            sleep_until: AtomicUsize::new(0),
            cwd: RwSpin::new(Cwd::new(root_dentry().clone())),
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

    pub fn new_param_kern(fun: usize, val: usize) -> Task {
        let mut task = Task::default();
        task.arch_task = UnsafeCell::new(ArchTask::new_param_kern(fun, val));
        task
    }

    pub fn new_user(fun: MappedAddr, code_size: usize) -> Task {
        let mut task = Task::default();
        task.arch_task = UnsafeCell::new(ArchTask::new_user(fun, code_size));

        task
    }

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        self.filetable.get_handle(fd)
    }

    pub fn get_dent(&self) -> DirEntryItem {
        self.cwd.read().dentry.clone()
    }

    pub fn get_pwd(&self) -> String {
        self.cwd.read().pwd()
    }

    pub fn open_file(&self, inode: DirEntryItem, flags: OpenFlags) -> Option<usize> {
        self.filetable.open_file(inode, flags)
    }

    pub fn close_file(&self, fd: usize) -> bool {
        self.filetable.close_file(fd)
    }

    pub fn close_all_files(&self) {
        self.filetable.close_all_files();
    }

    pub fn set_cwd(&self, dentry: DirEntryItem) {
        let mut cwd = self.cwd.write();

        if let Some(fs) = dentry.inode().fs().upgrade() {
            cwd.fs = fs;
            cwd.dentry = dentry;
        } else {
            println!("[ WARN ] CWD failed");
        }
    }

    pub fn set_state(&self, state: TaskState) {
        self.state.store(state as usize, Ordering::SeqCst);
    }

    pub fn to_reschedule(&self) -> bool {
        self.to_resched.load(Ordering::SeqCst)
    }

    pub fn set_to_reschedule(&self, s: bool) {
        self.to_resched.store(s, Ordering::SeqCst);
    }

    pub fn to_delete(&self) -> bool {
        self.to_delete.load(Ordering::SeqCst)
    }

    pub fn set_to_delete(&self, d: bool) {
        self.to_delete.store(d, Ordering::SeqCst);
    }

    pub fn halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }

    pub fn set_halted(&self, h: bool) {
        self.halted.store(h, Ordering::SeqCst);
    }

    pub fn state(&self) -> TaskState {
        if self.halted() {
            TaskState::Halted
        } else {
            self.state.load(Ordering::SeqCst).into()
        }
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

    pub fn has_pending_io(&self) -> bool {
        self.pending_io.load(Ordering::SeqCst)
    }

    pub fn set_has_pending_io(&self, has: bool) {
        self.pending_io.store(has, Ordering::SeqCst)
    }

    pub fn await_io(&self) {
        if self.has_pending_io() {
            self.set_has_pending_io(false);
        } else {
            self.sleep_until.store(0, Ordering::SeqCst);
            self.set_state(TaskState::AwaitingIo);
            crate::kernel::sched::reschedule();

            assert_eq!(self.state(), TaskState::Running);
        }
    }

    pub fn wake_up(&self) {
        let _ = self.state.compare_exchange(
            TaskState::AwaitingIo as usize,
            TaskState::Runnable as usize,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        self.set_halted(false);
    }

    pub unsafe fn arch_task_mut(&self) -> &mut ArchTask {
        &mut (*self.arch_task.get())
    }

    pub unsafe fn arch_task(&self) -> &ArchTask {
        &(*self.arch_task.get())
    }

    pub fn sleep(&self, time_ns: usize) {
        use crate::kernel::timer::current_ns;

        self.sleep_until
            .store(current_ns() as usize + time_ns, Ordering::SeqCst);
        self.set_state(TaskState::AwaitingIo);
        crate::kernel::sched::reschedule();
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            self.arch_task_mut().deallocate();
        }
    }
}
