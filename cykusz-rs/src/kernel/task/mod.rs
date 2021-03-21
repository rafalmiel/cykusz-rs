use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use intrusive_collections::LinkedListLink;

use syscall_defs::OpenFlags;

use crate::arch::mm::VirtAddr;
use crate::arch::task::Task as ArchTask;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::root_dentry;
use crate::kernel::sched::new_task_id;
use crate::kernel::signal::{SignalResult, Signals};
use crate::kernel::sync::{RwSpin, Spin};
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;
use crate::kernel::task::vm::{PageFaultReason, VM};
use crate::kernel::task::zombie::Zombies;
use crate::kernel::tty::Terminal;
use syscall_defs::signal::SIGCHLD;

pub mod cwd;
pub mod filetable;
pub mod vm;
pub mod zombie;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Runnable = 2,
    AwaitingIo = 4,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            2 => Runnable,
            4 => AwaitingIo,
            _ => unreachable!(),
        }
    }
}

intrusive_adapter!(pub TaskAdapter = Arc<Task> : Task { sibling: LinkedListLink });
intrusive_adapter!(pub SchedTaskAdapter = Arc<Task> : Task { sched: LinkedListLink });

#[derive(Default)]
pub struct Task {
    arch_task: UnsafeCell<ArchTask>,
    id: usize,
    on_cpu: AtomicUsize,
    parent: Spin<Option<Arc<Task>>>,
    children: Spin<intrusive_collections::LinkedList<TaskAdapter>>,
    sibling: intrusive_collections::LinkedListLink,
    pub sched: intrusive_collections::LinkedListLink,
    state: AtomicUsize,
    locks: AtomicUsize,
    pending_io: AtomicBool,
    to_resched: AtomicBool,
    filetable: filetable::FileTable,
    vm: VM,
    sleep_until: AtomicUsize,
    cwd: RwSpin<Option<Cwd>>,
    sref: Weak<Task>,
    signals: Arc<Signals>,
    terminal: Terminal,
    zombies: Zombies,
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Arc<Task> {
        Self::make_ptr(Self::new())
    }

    pub fn new() -> Task {
        Self::new_with_id(new_task_id())
    }

    pub fn new_with_id(id: usize) -> Task {
        let mut def = Task::default();

        def.id = id;
        def.on_cpu
            .store(unsafe { crate::CPU_ID } as usize, Ordering::SeqCst);

        if let Some(e) = root_dentry() {
            def.set_cwd(e.clone());
        }

        def.set_state(TaskState::Runnable);

        def
    }

    fn make_ptr(mut task: Task) -> Arc<Task> {
        Arc::new_cyclic(|me| {
            task.sref = me.clone();

            task.terminal.init(me);

            task
        })
    }

    pub fn new_sched(fun: fn()) -> Arc<Task> {
        let mut task = Task::new();
        task.arch_task = UnsafeCell::new(ArchTask::new_sched(fun));
        Self::make_ptr(task)
    }

    pub fn new_kern(fun: fn()) -> Arc<Task> {
        let mut task = Task::new();
        task.arch_task = UnsafeCell::new(ArchTask::new_kern(fun));
        Self::make_ptr(task)
    }

    pub fn new_param_kern(fun: usize, val: usize) -> Arc<Task> {
        let mut task = Task::new();
        task.arch_task = UnsafeCell::new(ArchTask::new_param_kern(fun, val));
        Self::make_ptr(task)
    }

    pub fn new_user(exe: DirEntryItem) -> Arc<Task> {
        let mut task = Task::new();

        let vm = task.vm();

        if let Some((entry, tls_vm)) = vm.load_bin(exe) {
            task.arch_task = UnsafeCell::new(ArchTask::new_user(entry, vm, tls_vm));

            Self::make_ptr(task)
        } else {
            panic!("Failed to exec task")
        }
    }

    pub fn fork(&self) -> Arc<Task> {
        let mut task = Task::new();

        task.arch_task = UnsafeCell::new(unsafe { self.arch_task().fork() });

        task.vm().fork(self.vm());
        task.filetable = self.filetable.clone();
        if let Some(e) = self.get_dent() {
            task.set_cwd(e);
        }
        task.set_state(TaskState::Runnable);
        task.set_locks(self.locks());

        let task = Self::make_ptr(task);

        task.set_parent(Some(self.me()));
        self.add_child(task.clone());

        self.terminal().try_transfer_to(task.clone());
        task.signals().copy_from(self.signals());

        task
    }

    pub fn exec(&self, exe: DirEntryItem) -> ! {
        let vm = self.vm();
        vm.clear();

        self.signals().clear();
        self.set_locks(0);

        if let Some((entry, tls_vm)) = vm.load_bin(exe) {
            unsafe { self.arch_task_mut().exec(entry, vm, tls_vm) }
        } else {
            panic!("Failed to exec task")
        }
    }

    fn me(&self) -> Arc<Task> {
        self.sref.upgrade().unwrap()
    }

    pub fn remove_child(&self, child: &Task) {
        let mut children = self.children.lock();

        if child.sibling.is_linked() {
            let mut cur = unsafe { children.cursor_mut_from_ptr(child) };

            child.set_parent(None);

            cur.remove();
        }
    }

    pub fn add_child(&self, child: Arc<Task>) {
        let mut children = self.children.lock();

        children.push_back(child);
    }

    pub fn get_parent(&self) -> Option<Arc<Task>> {
        let parent = self.parent.lock();

        parent.clone()
    }

    pub fn set_parent(&self, parent: Option<Arc<Task>>) {
        *self.parent.lock() = parent;
    }

    pub fn migrate_children_to(&self, to: Arc<Task>) {
        let mut old_list = self.children.lock();

        while let Some(child) = old_list.pop_back() {
            child.set_parent(Some(to.clone()));
            to.add_child(child);
        }

        if let Some(parent) = self.get_parent() {
            to.set_parent(Some(parent.clone()));
            parent.add_child(to);

            parent.remove_child(self);
            self.set_parent(None);
        }
    }

    pub fn migrate_children_to_parent(&self) {
        if let Some(parent) = self.get_parent() {
            let mut children = self.children.lock();

            while let Some(child) = children.pop_back() {
                child.set_parent(Some(parent.clone()));
                parent.add_child(child);
            }
        } else {
            let mut children = self.children.lock();

            for child in children.iter() {
                child.set_parent(None);
            }

            children.clear();
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

    pub fn state(&self) -> TaskState {
        self.state.load(Ordering::SeqCst).into()
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

    pub fn on_cpu(&self) -> usize {
        self.on_cpu.load(Ordering::SeqCst)
    }

    pub fn has_pending_io(&self) -> bool {
        self.pending_io.load(Ordering::SeqCst)
    }

    pub fn set_has_pending_io(&self, has: bool) {
        self.pending_io.store(has, Ordering::SeqCst);
    }

    pub fn await_io(&self) -> SignalResult<()> {
        let res = crate::kernel::sched::sleep(None);

        assert_eq!(
            self.state(),
            TaskState::Runnable,
            "await_io assert, id: {}",
            self.id()
        );

        res
    }

    pub fn wake_up(&self) {
        crate::kernel::sched::wake(self.me());
    }

    pub unsafe fn arch_task_mut(&self) -> &mut ArchTask {
        &mut (*self.arch_task.get())
    }

    pub unsafe fn arch_task(&self) -> &ArchTask {
        &(*self.arch_task.get())
    }

    pub fn sleep(&self, time_ns: usize) -> SignalResult<()> {
        crate::kernel::sched::sleep(Some(time_ns))
    }

    pub fn sleep_until(&self) -> usize {
        self.sleep_until.load(Ordering::SeqCst)
    }

    pub fn set_sleep_until(&self, val: usize) {
        self.sleep_until.store(val, Ordering::SeqCst);
    }

    pub fn handle_pagefault(&self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        self.vm.handle_pagefault(reason, addr)
    }

    pub fn vm(&self) -> &VM {
        &self.vm
    }

    pub fn signals(&self) -> &Arc<Signals> {
        &self.signals
    }

    pub fn signal(&self, sig: usize) {
        if self.signals.trigger(sig) {
            self.wake_up();
        }
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn make_zombie(&self) {
        if let Some(parent) = self.get_parent() {
            parent.zombies.add_zombie(self.me());

            parent.remove_child(self);
            self.set_parent(None);

            self.terminal().try_transfer_to(parent.clone());

            parent.signal(SIGCHLD);
        }

        unsafe {
            self.arch_task_mut().deallocate();
        }
    }

    pub fn wait_pid(&self, pid: usize) -> SignalResult<usize> {
        self.zombies.wait_pid(pid)
    }
}

impl Drop for Task {
    fn drop(&mut self) {}
}
