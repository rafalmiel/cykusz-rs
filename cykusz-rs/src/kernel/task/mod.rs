use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use intrusive_collections::{LinkedList, LinkedListLink};

use syscall_defs::exec::ExeArgs;
use syscall_defs::signal::SIGCHLD;
use syscall_defs::{OpenFlags, SyscallError, SyscallResult};

use crate::arch::mm::VirtAddr;
use crate::arch::task::Task as ArchTask;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::root_dentry;
use crate::kernel::sched::{new_task_tid, SleepFlags};
use crate::kernel::signal::{SignalResult, Signals, KSIGSTOPTHR};
use crate::kernel::sync::{LockApi, RwSpin, Spin, SpinGuard};
use crate::kernel::task::children_events::WaitPidEvents;
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;
use crate::kernel::task::vm::{PageFaultReason, VM};
use crate::kernel::tty::Terminal;

pub mod children_events;
pub mod cwd;
pub mod filetable;
pub mod vm;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Runnable = 2,
    AwaitingIo = 4,
    Stopped = 5,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            2 => Runnable,
            4 => AwaitingIo,
            5 => Stopped,
            _ => panic!("Invalid task state: {}", val),
        }
    }
}

intrusive_adapter!(pub TaskAdapter = Arc<Task> : Task { sibling: LinkedListLink });
intrusive_adapter!(pub SchedTaskAdapter = Arc<Task> : Task { sched: LinkedListLink });
intrusive_adapter!(pub WaitPidTaskAdapter = Arc<Task> : Task { waitpid: LinkedListLink });

#[derive(Default)]
pub struct Task {
    arch_task: UnsafeCell<ArchTask>,
    tid: usize,
    pid: usize,
    gid: AtomicUsize,
    sid: AtomicUsize,
    on_cpu: AtomicUsize,
    parent: Spin<Option<Arc<Task>>>,
    children: Spin<intrusive_collections::LinkedList<TaskAdapter>>,
    sibling: intrusive_collections::LinkedListLink,
    pub sched: intrusive_collections::LinkedListLink,
    pub waitpid: intrusive_collections::LinkedListLink,
    state: AtomicUsize,
    locks: AtomicUsize,
    pending_io: AtomicBool,
    to_resched: AtomicBool,
    filetable: Arc<filetable::FileTable>,
    vm: Arc<VM>,
    sleep_until: AtomicUsize,
    cwd: RwSpin<Option<Cwd>>,
    sref: Weak<Task>,
    signals: Signals,
    terminal: Terminal,
    children_events: WaitPidEvents,
    waitpid_status: AtomicU64,
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> Arc<Task> {
        Self::make_ptr(Self::new())
    }

    pub fn new() -> Task {
        Self::new_with_id(new_task_tid())
    }

    pub fn new_with_id(tid: usize) -> Task {
        let mut def = Task::default();

        def.set_tid(tid);
        def.set_pid(tid);
        def.set_gid(tid);
        def.set_sid(tid);

        def.on_cpu
            .store(unsafe { crate::CPU_ID } as usize, Ordering::SeqCst);

        if let Some(e) = root_dentry() {
            def.set_cwd(e.clone());
        }

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

    pub fn fork(&self) -> Arc<Task> {
        let mut task = Task::new();

        task.set_gid(self.gid());
        task.set_sid(self.sid());

        task.arch_task = UnsafeCell::new(unsafe { self.arch_task().fork() });

        task.vm().fork(self.vm());
        task.filetable = Arc::new(self.filetable.as_ref().clone());
        if let Some(e) = self.get_dent() {
            task.set_cwd(e);
        }
        task.set_locks(self.locks());

        let task = Self::make_ptr(task);

        self.add_child(task.clone());

        task.signals().copy_from(self.signals());

        if let Some(term) = self.terminal().terminal() {
            task.terminal().connect(term);
        }

        logln2!("new fork task {}", task.pid());

        task
    }

    pub fn exec(
        &self,
        mut exe: DirEntryItem,
        args: Option<ExeArgs>,
        envs: Option<ExeArgs>,
    ) -> Result<!, SyscallError> {
        logln5!("exec task {} {}", self.pid(), exe.full_path());

        let mut args = args.unwrap_or(ExeArgs::new());

        let vm = VM::new();

        let (base_addr, entry, elf_hdr, tls_vm, interpreter) =
            vm.load_bin(exe.clone()).ok_or(SyscallError::EINVAL)?;

        if let Some((interp, additional_args)) = interpreter {
            // got a shebang interpreter line?? replace exe and pass script as a first param
            args.push_front(Box::from(exe.full_path().as_bytes()));

            if let Some(aa) = additional_args {
                for (idx, a) in aa.iter().enumerate() {
                    args.insert(idx + 1, a.clone());
                }
            }
            exe = interp;
        }
        vm.log_vm();

        // Replace our new vm
        self.vm.fork(&vm);

        // New process does not inherits signals
        self.signals().clear();

        // No locks
        self.set_locks(0);

        // Close all files with CLOEXEC flags
        self.filetable().close_on_exec();
        self.filetable().debug();

        // !!! Prevent memory leak as we are not running destructors here!
        drop(vm);

        unsafe {
            // EXEC!
            self.arch_task_mut().exec(
                base_addr,
                entry,
                &elf_hdr,
                self.vm(),
                tls_vm,
                exe,
                Some(args),
                envs,
            )
        }
    }

    pub fn spawn_thread(&self, entry: VirtAddr, user_stack: VirtAddr) -> Arc<Task> {
        let mut thread = Task::new();

        thread.arch_task =
            UnsafeCell::new(unsafe { self.arch_task().fork_thread(entry.0, user_stack.0) });

        let process_leader = if self.is_process_leader() {
            self.me()
        } else {
            let process_leader = self
                .get_parent()
                .expect("Not a group leader parent missing?");

            assert!(
                process_leader.is_process_leader(),
                "Parent not a process leader?"
            );
            process_leader
        };

        thread.set_pid(process_leader.pid());
        thread.set_gid(process_leader.gid());
        thread.set_sid(process_leader.sid());

        thread.filetable = process_leader.filetable.clone();
        thread.vm = process_leader.vm.clone();
        if let Some(d) = process_leader.get_dent() {
            thread.set_cwd(d);
        }
        thread.signals = process_leader.signals().clone();

        self.terminal().share_with(&mut thread.terminal);

        let thread = Self::make_ptr(thread);

        logln_disabled!("set parent {} -> {}", process_leader.tid(), thread.tid());
        process_leader.add_child(thread.clone());

        thread
    }

    pub fn me(&self) -> Arc<Task> {
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

    pub fn remove_from_parent(&self) {
        if let Some(parent) = self.get_parent() {
            parent.remove_child(self);
        }
    }

    pub fn add_child(&self, child: Arc<Task>) {
        let mut children = self.children.lock();

        child.set_parent(Some(self.me()));

        children.push_back(child);
    }

    pub fn get_parent(&self) -> Option<Arc<Task>> {
        let parent = self.parent.lock();

        parent.clone()
    }

    pub fn set_parent(&self, parent: Option<Arc<Task>>) {
        *self.parent.lock() = parent;
    }

    pub fn children(&self) -> SpinGuard<LinkedList<TaskAdapter>> {
        self.children.lock()
    }

    pub fn migrate_children_to_init(&self) {
        let parent = crate::kernel::init::init_task();

        let mut children = self.children.lock_irq();

        let mut cursor = children.front_mut();

        while let Some(child) = cursor.get() {
            if child.is_process_leader() {
                let task = cursor.remove().unwrap();

                dbgln!(task, "Move task {} to init", task.tid());

                parent.add_child(task);
            } else {
                cursor.move_next();
            }
        }

        // Migrate already dead child processes not waited for by the parent
        parent.children_events.migrate(&self.children_events);
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

    pub fn open_file(
        &self,
        inode: DirEntryItem,
        flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        self.filetable.open_file(inode, flags)
    }

    pub fn filetable(&self) -> &filetable::FileTable {
        &self.filetable
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

    pub fn waitpid_status(&self) -> syscall_defs::waitpid::Status {
        self.waitpid_status.load(Ordering::SeqCst).into()
    }

    pub fn set_waitpid_status(&self, status: syscall_defs::waitpid::Status) {
        self.waitpid_status.store(status.into(), Ordering::SeqCst);
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

    pub fn dec_locks(&self) -> usize {
        self.locks.fetch_sub(1, Ordering::SeqCst) - 1
    }

    pub fn set_locks(&self, locks: usize) {
        self.locks.store(locks, Ordering::SeqCst);
    }

    pub fn locks(&self) -> usize {
        self.locks.load(Ordering::SeqCst)
    }

    pub fn locks_ref(&self) -> &AtomicUsize {
        &self.locks
    }

    pub fn tid(&self) -> usize {
        self.tid
    }

    pub fn set_tid(&mut self, v: usize) {
        self.tid = v;
    }

    pub fn pid(&self) -> usize {
        self.pid
    }

    pub fn set_pid(&mut self, v: usize) {
        self.pid = v;
    }

    pub fn gid(&self) -> usize {
        self.gid.load(Ordering::SeqCst)
    }

    pub fn set_gid(&self, v: usize) {
        self.gid.store(v, Ordering::SeqCst);
    }

    pub fn sid(&self) -> usize {
        self.sid.load(Ordering::SeqCst)
    }

    pub fn set_sid(&self, v: usize) {
        self.sid.store(v, Ordering::SeqCst)
    }

    pub fn is_process_leader(&self) -> bool {
        self.tid() == self.pid()
    }

    pub fn process_leader(&self) -> Arc<Task> {
        if self.is_process_leader() {
            self.me()
        } else {
            let parent = self.get_parent().unwrap();

            assert!(parent.is_process_leader());

            parent
        }
    }

    pub fn is_group_leader(&self) -> bool {
        self.gid() == self.pid()
    }

    pub fn is_session_leader(&self) -> bool {
        self.pid() == self.sid()
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

    pub fn stop_threads(&self) {
        assert!(self.is_process_leader());

        let mut count = 0;

        for c in self
            .children()
            .iter()
            .filter(|t| t.pid() == self.pid() && t.state() != TaskState::Stopped)
        {
            c.signal_thread(KSIGSTOPTHR);
            count += 1;
        }

        while count > 0 {
            match self.wait_thread(0, syscall_defs::waitpid::WaitPidFlags::STOPPED, true) {
                Ok(Err(SyscallError::ECHILD)) => {
                    break;
                }
                Ok(Ok(_tid)) => {
                    count -= 1;
                }
                Err(_) => {
                    continue;
                }
                Ok(Err(e)) => {
                    panic!("Unexpected error from wait_thread: {:?}", e);
                }
            }
        }
    }

    pub fn cont_threads(&self) {
        assert!(self.is_process_leader());

        let mut count = 0;

        for c in self
            .children()
            .iter()
            .filter(|t| t.pid() == self.pid() && t.state() == TaskState::Stopped)
        {
            crate::kernel::sched::cont_thread(c.me());
            count += 1;
        }

        while count > 0 {
            match self.wait_thread(
                0,
                syscall_defs::waitpid::WaitPidFlags::EXITED
                    | syscall_defs::waitpid::WaitPidFlags::CONTINUED,
                true,
            ) {
                Ok(Err(SyscallError::ECHILD)) => {
                    break;
                }
                Ok(Ok(_tid)) => {
                    count -= 1;
                }
                Err(_) => {
                    continue;
                }
                Ok(Err(e)) => {
                    panic!("Unexpected error from wait_thread: {:?}", e);
                }
            }
        }
    }

    pub fn terminate_threads(&self) {
        assert!(self.is_process_leader());

        for c in self
            .children()
            .iter()
            .filter(|t| t.pid() == self.pid() && t.state() != TaskState::Unused)
        {
            assert!(!c.is_process_leader());
            c.signal_thread(crate::kernel::signal::KSIGKILLTHR);
        }

        loop {
            match self.wait_thread(0, syscall_defs::waitpid::WaitPidFlags::EXITED, true) {
                Ok(Err(SyscallError::ECHILD)) => {
                    break;
                }
                Ok(Ok(_tid)) => {}
                Err(_) => {
                    continue;
                }
                Ok(Err(e)) => {
                    panic!("Unexpected error from wait_thread: {:?}", e);
                }
            }
        }
    }

    fn do_await_io(&self, timeout_ns: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
        if self.locks() > 0 {
            logln!(
                "await_io: sleeping while holding locks: {}, tid {}",
                self.locks(),
                self.tid()
            );
        }
        let res = crate::kernel::sched::sleep(timeout_ns, flags);

        assert_eq!(
            self.state(),
            TaskState::Runnable,
            "await_io assert, id: {}",
            self.tid()
        );

        res
    }

    pub fn await_io(&self, flags: SleepFlags) -> SignalResult<()> {
        self.do_await_io(None, flags)
    }

    pub fn await_io_timeout(
        &self,
        timeout_ns: Option<usize>,
        flags: SleepFlags,
    ) -> SignalResult<()> {
        self.do_await_io(timeout_ns, flags)
    }

    pub fn wake_up(&self) {
        crate::kernel::sched::wake(self.me());
    }

    pub fn wake_up_as_next(&self) {
        crate::kernel::sched::wake_as_next(self.me());
    }

    pub unsafe fn arch_task_mut(&self) -> &mut ArchTask {
        &mut (*self.arch_task.get())
    }

    pub unsafe fn arch_task(&self) -> &ArchTask {
        &(*self.arch_task.get())
    }

    pub fn sleep(&self, time_ns: usize) -> SignalResult<()> {
        crate::kernel::sched::sleep(Some(time_ns), SleepFlags::empty())
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

    pub fn vm(&self) -> &Arc<VM> {
        &self.vm
    }

    pub fn signals(&self) -> &Signals {
        &self.signals
    }

    fn do_signal(&self, sig: usize, this_thread: bool) -> bool {
        use crate::kernel::signal::TriggerResult;

        match self.signals().trigger(sig, this_thread) {
            TriggerResult::Triggered => {
                self.wake_up();

                true
            }
            TriggerResult::Execute(f) => {
                logln2!("SIG TRIGGER EXECUTE {}", sig);
                f(sig, self.me());

                true
            }
            TriggerResult::Blocked if !this_thread => {
                // Find other thread in process to notify
                let process_leader = self.process_leader();

                if !process_leader.signals().is_blocked(sig) {
                    process_leader.wake_up();

                    return true;
                }

                for c in process_leader
                    .children()
                    .iter()
                    .filter(|t| t.pid() == self.pid())
                {
                    if !c.signals().is_blocked(sig) {
                        c.wake_up();

                        return true;
                    }
                }

                false
            }
            TriggerResult::Blocked => false,
        }
    }

    pub fn signal(&self, sig: usize) -> bool {
        logln5!("signal {} sig: {}", self.pid(), sig);
        if self.state() != TaskState::Unused {
            self.do_signal(sig, false)
        } else {
            false
        }
    }

    pub fn signal_thread(&self, sig: usize) -> bool {
        self.do_signal(sig, true)
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn make_zombie(&self, status: syscall_defs::waitpid::Status) {
        self.terminal().disconnect(None);

        unsafe {
            self.arch_task_mut().deallocate();
        }

        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(status);
            parent.children_events.add_zombie(self.me());

            if self.is_process_leader() {
                parent.signal(SIGCHLD);
            }
        }
    }
    pub fn notify_continued(&self) {
        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(syscall_defs::waitpid::Status::Continued);
            parent.children_events.add_continued(self.me());

            if self.is_process_leader() {
                parent.signal(SIGCHLD);
            }
        }
    }

    pub fn notify_stopped(&self, sig: usize) {
        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(syscall_defs::waitpid::Status::Stopped(sig as u64));
            parent.children_events.add_stopped(self.me());

            if self.is_process_leader() {
                parent.signal(SIGCHLD);
            }
        }
    }

    pub fn wait_pid(
        &self,
        pid: isize,
        status: &mut syscall_defs::waitpid::Status,
        flags: syscall_defs::waitpid::WaitPidFlags,
    ) -> SignalResult<SyscallResult> {
        self.children_events.wait_pid(self, pid, status, flags)
    }

    pub fn wait_thread(
        &self,
        tid: usize,
        flags: syscall_defs::waitpid::WaitPidFlags,
        no_intr: bool,
    ) -> SignalResult<SyscallResult> {
        self.children_events
            .wait_thread(self, self.pid(), tid, flags, no_intr)
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        logln!("drop task {}", self.tid());
    }
}
