use alloc::string::String;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use syscall_defs::OpenFlags;

use crate::arch::mm::VirtAddr;
use crate::arch::task::Task as ArchTask;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::root_dentry;
use crate::kernel::sched::new_task_id;
use crate::kernel::sync::RwSpin;
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;
use crate::kernel::task::vm::{PageFaultReason, VM};

pub mod cwd;
pub mod filetable;
pub mod vm;

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
    vm: VM,
    pub sleep_until: AtomicUsize,
    cwd: RwSpin<Option<Cwd>>,
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
            vm: VM::new(),
            sleep_until: AtomicUsize::new(0),
            cwd: RwSpin::new(if let Some(e) = root_dentry() {
                Cwd::new(e.clone())
            } else {
                None
            }),
        }
    }
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Task {
        Task::default()
    }

    pub fn default_with_id(id: usize) -> Task {
        Task {
            arch_task: UnsafeCell::new(ArchTask::empty()),
            id,
            state: AtomicUsize::new(TaskState::Runnable as usize),
            locks: AtomicUsize::new(0),
            pending_io: AtomicBool::new(false),
            to_resched: AtomicBool::new(false),
            to_delete: AtomicBool::new(false),
            halted: AtomicBool::new(false),
            filetable: filetable::FileTable::new(),
            vm: VM::new(),
            sleep_until: AtomicUsize::new(0),
            cwd: RwSpin::new(if let Some(e) = root_dentry() {
                Cwd::new(e.clone())
            } else {
                None
            }),
        }
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

    pub fn new_user(exe: DirEntryItem) -> Task {
        let mut task = Task::default();

        let vm = task.vm();

        if let Some(entry) = vm.load_bin(exe) {
            task.arch_task = UnsafeCell::new(ArchTask::new_user(entry, vm));

            task
        } else {
            panic!("Failed to exec task")
        }
    }

    pub fn fork(&self) -> Task {
        let mut task = Task::default();

        task.arch_task = UnsafeCell::new(unsafe { self.arch_task().fork() });

        task.vm().fork(self.vm());
        task.filetable = self.filetable.clone();
        if let Some(e) = self.get_dent() {
            task.set_cwd(e);
        }
        task.set_state(TaskState::Runnable);
        task.set_locks(self.locks());

        task
    }

    pub fn exec(&self, exe: DirEntryItem) -> Task {
        //println!("execing id {}", self.id);
        let mut task = Task::default_with_id(self.id);

        let vm = task.vm();

        if let Some(entry) = vm.load_bin(exe) {
            task.arch_task = UnsafeCell::new(ArchTask::new_user(entry, vm));

            task.filetable = self.filetable.clone();
            if let Some(e) = self.get_dent() {
                task.set_cwd(e);
            }

            task
        } else {
            panic!("Failed to exec task")
        }
    }

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        self.filetable.get_handle(fd)
    }

    pub fn get_dent(&self) -> Option<DirEntryItem> {
        let cwd = self.cwd.read();

        Some((*cwd).as_ref()?.dentry.clone())
    }

    pub fn get_pwd(&self) -> Option<String> {
        let cwd = self.cwd.read();

        Some((*cwd).as_ref()?.pwd())
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

    pub fn clear_cwd(&self) {
        let mut cwd = self.cwd.write();

        *cwd = None;
    }

    pub fn set_cwd(&self, dentry: DirEntryItem) {
        let mut cwd = self.cwd.write();

        *cwd = Cwd::new(dentry);

        if cwd.is_none() {
            println!("[ WARN ] Cwd failed");
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

    pub fn inc_locks(&self) {
        self.locks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn dec_locks(&self) {
        self.locks.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn set_locks(&self, locks: usize) {
        self.locks.store(locks, Ordering::SeqCst);
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

    pub fn handle_pagefault(&self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        self.vm.handle_pagefault(reason, addr)
    }

    pub fn vm(&self) -> &VM {
        &self.vm
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            self.arch_task_mut().deallocate();
        }
    }
}
