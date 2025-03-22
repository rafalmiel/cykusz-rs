use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
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
use crate::kernel::sched::{current_task_ref, new_task_tid, SleepFlags};
use crate::kernel::signal::{SignalResult, Signals, KSIGSTOPTHR};
use crate::kernel::sync::{LockApi, RwSpin, Spin, SpinGuard};
use crate::kernel::task::children_events::WaitPidEvents;
use crate::kernel::task::cwd::Cwd;
use crate::kernel::task::filetable::FileHandle;
use crate::kernel::task::vm::{PageFaultReason, VM};
use crate::kernel::tty::Terminal;
use crate::kernel::utils::arc_type::{ArcType, Uid, WeakType};

pub mod children_events;
pub mod cwd;
pub mod filetable;
pub mod vm;
#[macro_use]
pub mod intrusive_adapter;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskState {
    Unused = 0,
    Idle = 1,
    Runnable = 2,
    AwaitingIo = 4,
    Stopped = 5,
}

impl From<usize> for TaskState {
    fn from(val: usize) -> Self {
        use self::TaskState::*;

        match val {
            0 => Unused,
            1 => Idle,
            2 => Runnable,
            4 => AwaitingIo,
            5 => Stopped,
            _ => panic!("Invalid task state: {}", val),
        }
    }
}

pub type ArcTask = ArcType<Task>;
pub type WeakTask = WeakType<Task>;

impl Uid for Task {
    fn uid(&self) -> usize {
        self.tid()
    }
}

task_intrusive_adapter!(pub TaskAdapter = ArcType<Task> : Task { sibling: LinkedListLink });
task_intrusive_adapter!(pub SchedTaskAdapter = ArcType<Task> : Task { sched: LinkedListLink });
task_intrusive_adapter!(pub WaitPidTaskAdapter = ArcType<Task> : Task { waitpid: LinkedListLink });

//intrusive_adapter!(pub TaskAdapter = Arc<Task> : Task { sibling: LinkedListLink });
//intrusive_adapter!(pub SchedTaskAdapter = Arc<Task> : Task { sched: LinkedListLink });
//intrusive_adapter!(pub WaitPidTaskAdapter = Arc<Task> : Task { waitpid: LinkedListLink });

#[derive(Default)]
pub struct Task {
    arch_task: UnsafeCell<ArchTask>,
    tid: usize,
    pid: usize,
    gid: AtomicUsize,
    sid: AtomicUsize,
    on_cpu: AtomicUsize,
    exe: Spin<Option<DirEntryItem>>,
    parent: Spin<Option<ArcTask>>,
    children: Spin<intrusive_collections::LinkedList<TaskAdapter>>,
    sibling: intrusive_collections::LinkedListLink,
    pub sched: intrusive_collections::LinkedListLink,
    pub waitpid: intrusive_collections::LinkedListLink,
    state: AtomicUsize,
    locks: AtomicUsize,
    pending_io: AtomicBool,
    to_resched: AtomicBool,
    terminating: AtomicBool,
    filetable: Arc<filetable::FileTable>,
    vm: Arc<VM>,
    sleep_until: AtomicUsize,
    cwd: RwSpin<Option<Cwd>>,
    sref: WeakTask,
    signals: Signals,
    terminal: Terminal,
    children_events: WaitPidEvents,
    waitpid_status: AtomicU64,
}

unsafe impl Sync for Task {}

impl Task {
    pub fn this() -> ArcTask {
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

    fn make_ptr(mut task: Task) -> ArcTask {
        ArcTask::new_cyclic(|me| {
            task.sref = WeakTask::from(me.clone());

            task.terminal.init(&task.sref);

            task
        })
    }

    pub fn new_sched(fun: fn()) -> ArcTask {
        let mut task = Task::new();
        task.arch_task = UnsafeCell::new(ArchTask::new_sched(fun));
        Self::make_ptr(task)
    }

    pub fn new_kern(fun: fn()) -> ArcTask {
        let task = ArcTask::new_cyclic(|me| {
            let mut task = Task::new();
            task.sref = WeakTask::from(me.clone());
            task.arch_task = UnsafeCell::new(ArchTask::new_kern(fun));
            task.terminal.init(&task.sref);
            task
        });

        task
    }

    pub fn new_param_kern(fun: usize, val: usize) -> ArcTask {
        let mut task = Task::new();
        task.arch_task = UnsafeCell::new(ArchTask::new_param_kern(fun, val));
        Self::make_ptr(task)
    }

    pub fn exe(&self) -> Option<DirEntryItem> {
        self.exe.lock().clone()
    }

    pub fn set_exe(&self, exe: Option<DirEntryItem>) {
        *self.exe.lock() = exe;
    }

    pub fn fork(&self) -> ArcTask {
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

        task.set_exe(self.exe());

        logln2!("new fork task {}", task.pid());

        task
    }

    pub fn exec(
        &self,
        mut exe: DirEntryItem,
        args: Option<ExeArgs>,
        envs: Option<ExeArgs>,
    ) -> Result<!, SyscallError> {
        dbgln!(task, "exec task {} {}", self.pid(), exe.full_path());

        self.set_terminating(false);

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

        dbgln!(task, "vm forked {}", self.tid());

        // New process does not inherits signals
        self.signals().clear();

        // No locks
        self.set_locks(0);

        // Close all files with CLOEXEC flags
        self.filetable().close_on_exec();
        self.filetable().debug();

        // !!! Prevent memory leak as we are not running destructors here!
        drop(vm);

        dbgln!(
            task,
            "exec {} {} {} sc: {}, wc: {}",
            exe.full_path(),
            exe.name(),
            exe.parent().is_some(),
            ArcTask::strong_count(&self.me()) - 1,
            ArcTask::weak_count(&self.me()),
        );

        self.set_exe(Some(exe.clone()));

        unsafe {
            // EXEC!
            dbgln!(task, "task arch exec {}", self.tid());
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

    pub fn spawn_thread(&self, entry: VirtAddr, user_stack: VirtAddr) -> ArcTask {
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

    pub fn me(&self) -> ArcTask {
        if let Some(t) = self.sref.upgrade() {
            t
        } else {
            dbgln!(sched_v, "Task::me == nullptr");
            crate::lang_items::print_current_backtrace();
            loop {}
        }
    }

    pub fn remove_child(&self, child: &Task) {
        if child.sibling.is_linked() {
            let mut children = self.children_debug(9);

            let mut cur = unsafe { children.cursor_mut_from_ptr(child) };

            child.set_parent(None);

            cur.remove();
            dbgln!(
                task,
                "remove child {} from parent {}",
                child.tid(),
                self.tid()
            );
        } else {
            dbgln!(
                task,
                "remove child failed {}, p: {}",
                child.tid(),
                self.tid()
            );
        }

        let children = self.children_debug(10);
        for child in children.iter() {
            dbgln!(task, "remaining child {} {:?}", child.tid(), child.state());
        }
    }

    pub fn remove_from_parent(&self) {
        if let Some(parent) = self.get_parent() {
            dbgln!(
                task,
                "task {} remove from parent {}",
                self.tid(),
                parent.tid()
            );
            parent.remove_child(self);
        } else {
            dbgln!(task, "task {} remove from parent NOT FOUND", self.tid());
        }
    }

    pub fn add_child(&self, child: ArcTask) {
        child.set_parent(Some(self.me()));

        let mut children = self.children_debug(11);
        children.push_back(child.clone());
        dbgln!(task, "add child {} to parent {}", child.tid(), self.tid());
    }

    pub fn get_parent(&self) -> Option<ArcTask> {
        let parent = self.parent.lock();

        parent.clone()
    }

    pub fn set_parent(&self, parent: Option<ArcTask>) {
        *self.parent.lock() = parent;
    }

    pub fn children(&self) -> SpinGuard<'_, LinkedList<TaskAdapter>> {
        self.children.lock()
    }

    pub fn children_debug(&self, dbg: usize) -> SpinGuard<'_, LinkedList<TaskAdapter>> {
        self.children.lock_irq_debug(dbg)
    }

    pub fn migrate_children_to_init(&self) {
        let parent = crate::kernel::init::init_task();

        dbgln!(
            task,
            "migrate to init, interrupts: {}",
            crate::kernel::int::is_enabled()
        );

        let mut children = self.children_debug(1);

        let mut cursor = children.front_mut();

        while let Some(child) = cursor.get() {
            if child.is_process_leader() {
                let task = ArcTask::from(cursor.remove().unwrap());

                dbgln!(task, "Move task {} to init", task.tid());

                parent.add_child(task);
            } else {
                cursor.move_next();
            }
        }

        drop(children);
        dbgln!(
            task,
            "migrate to init fini, interrupts: {}",
            crate::kernel::int::is_enabled()
        );

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

    pub fn is_terminatng(&self) -> bool {
        self.terminating.load(Ordering::Relaxed)
    }

    pub fn set_terminating(&self, t: bool) {
        self.terminating.store(t, Ordering::Relaxed)
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

    pub fn is_parent_terminating(&self) -> bool {
        if self.is_process_leader() {
            false
        } else {
            if let Some(p) = self.get_parent() {
                p.is_terminatng()
            } else {
                false
            }
        }
    }

    pub fn process_leader(&self) -> ArcTask {
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
        self.on_cpu.load(Ordering::Relaxed)
    }

    pub fn is_on_this_cpu(&self) -> bool {
        crate::cpu_id() as usize == self.on_cpu()
    }

    pub fn set_on_cpu(&self, cpu: usize) {
        self.on_cpu.store(cpu, Ordering::Relaxed);
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
            .children_debug(3)
            .iter()
            .filter(|t| t.pid() == self.pid() && t.state() != TaskState::Stopped)
        {
            dbgln!(task_stop2, "stop thread {}", c.tid());
            c.signal_thread(KSIGSTOPTHR);
            count += 1;
        }

        while count > 0 {
            match self.wait_thread(0, syscall_defs::waitpid::WaitPidFlags::STOPPED, true) {
                Ok(Err(SyscallError::ECHILD)) => {
                    break;
                }
                Ok(Ok(_tid)) => {
                    dbgln!(task_stop2, "thread notified stopped {}", _tid);
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

        for c in self.children_debug(4).iter().filter(|t| {
            !t.is_process_leader() && t.pid() == self.pid() && t.state() == TaskState::Stopped
        }) {
            dbgln!(task_stop2, "cont thread {}", c.tid());
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
                    dbgln!(task_stop2, "thread notified cont {}", _tid);
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

    pub fn terminate_children(&self) {
        dbgln!(waitpid, "terminate {} children!", self.tid());
        'outer: loop {
            let mut to_kill = Vec::<ArcTask>::new();
            for c in self.children_debug(5).iter().filter(|t| {
                t.is_process_leader() && t.pid() != self.pid() && t.state() != TaskState::Unused
            }) {
                dbgln!(
                    poweroff,
                    "sigkill to {} {}",
                    c.pid(),
                    if let Some(e) = c.exe() {
                        e.full_path()
                    } else {
                        String::new()
                    }
                );
                to_kill.push(c.me());
            }

            for t in &to_kill {
                t.signal(syscall_defs::signal::SIGKILL);
            }

            to_kill.clear();

            'inner: loop {
                let mut status = syscall_defs::waitpid::Status::Invalid(0);
                match self.wait_pid(
                    -1,
                    &mut status,
                    syscall_defs::waitpid::WaitPidFlags::EXITED
                        | syscall_defs::waitpid::WaitPidFlags::CONTINUED
                        | syscall_defs::waitpid::WaitPidFlags::STOPPED,
                ) {
                    Ok(Err(SyscallError::ECHILD)) => {
                        if self
                            .children_debug(13)
                            .iter()
                            .any(|t| t.is_process_leader())
                        {
                            dbgln!(poweroff, "got ECHILD with children");
                            break 'inner;
                        } else {
                            dbgln!(poweroff, "got ECHILD without children");
                            break 'outer;
                        }
                    }
                    Ok(Ok(pid)) => {
                        dbgln!(
                            poweroff,
                            "terminated child pid: {} status: {:?}",
                            pid,
                            status
                        );
                    }
                    Err(e) => {
                        let empty = !self.children_debug(14).is_empty();
                        dbgln!(
                            poweroff,
                            "got signal error {:?}, pending: {:#x} has_children: {}",
                            e,
                            self.signals().pending(),
                            empty
                        );
                        self.signals().clear_pending(SIGCHLD as u64);
                        break 'inner;
                    }
                    Ok(Err(e)) => {
                        panic!("Unexpected error from wait_pid {:?}", e);
                    }
                }
            }
        }
        dbgln!(waitpid, "terminate {} children FINISHED!", self.tid());
    }

    pub fn terminate_threads(&self) {
        self.set_terminating(true);
        dbgln!(waitpid, "terminate threads!");
        'outer: loop {
            for c in self.children_debug(6).iter().filter(|t| {
                !t.is_process_leader() && t.pid() == self.pid() && t.state() != TaskState::Unused
            }) {
                dbgln!(
                    task,
                    "sigkillthr to {} {}",
                    c.tid(),
                    if let Some(e) = c.exe() {
                        e.full_path()
                    } else {
                        String::new()
                    }
                );
                c.signal_thread(crate::kernel::signal::KSIGKILLTHR);
            }

            'inner: loop {
                match self.wait_thread(0, syscall_defs::waitpid::WaitPidFlags::EXITED, true) {
                    Ok(Err(SyscallError::ECHILD)) => {
                        if self
                            .children
                            .lock()
                            .iter()
                            .any(|t| !t.is_process_leader() && t.pid() == self.pid())
                        {
                            dbgln!(poweroff, "got ECHILD with children");
                            break 'inner;
                        } else {
                            dbgln!(poweroff, "got ECHILD without children");
                            break 'outer;
                        }
                    }
                    Ok(Ok(pid)) => {
                        dbgln!(poweroff, "terminated child pid: {}", pid);
                    }
                    Err(e) => {
                        let empty = !self.children_debug(15).is_empty();
                        dbgln!(
                            poweroff,
                            "got signal error {:?}, pending: {:#x} has_children: {}",
                            e,
                            self.signals().pending(),
                            !empty
                        );
                    }
                    Ok(Err(e)) => {
                        panic!("Unexpected error from wait_pid {:?}", e);
                    }
                }
            }
        }
        dbgln!(waitpid, "terminate threads FINISHED!");
    }

    pub fn terminate_threads2(&self) {
        assert!(self.is_process_leader());

        dbgln!(task, "teminate threads {}", self.tid());

        loop {
            let mut found_threads = false;

            for c in self
                .children_debug(7)
                .iter()
                .filter(|t| t.pid() == self.pid() && t.state() != TaskState::Unused)
            {
                assert!(!c.is_process_leader());
                dbgln!(task, "signal KILLTHR {}", c.tid());
                c.signal_thread(crate::kernel::signal::KSIGKILLTHR);
                found_threads = true;
            }

            if !found_threads {
                return;
            }

            loop {
                match self.wait_thread(0, syscall_defs::waitpid::WaitPidFlags::EXITED, true) {
                    Ok(Err(SyscallError::ECHILD)) => {
                        dbgln!(task, "terminate_threads ECHILD");
                        break;
                    }
                    Ok(Ok(_tid)) => {
                        dbgln!(task, "terminate_threads OK task {}", _tid);
                    }
                    Err(_) => {
                        dbgln!(task, "terminate_threads ERR");
                        continue;
                    }
                    Ok(Err(e)) => {
                        panic!("Unexpected error from wait_thread: {:?}", e);
                    }
                }
            }
        }
    }

    fn do_await_io(&self, timeout_ns: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
        //dbgln!(task, "{} do_await_io {:?}", self.tid(), self.state());
        if self.locks() > 0 {
            logln!(
                "await_io: sleeping while holding locks: {}, tid {}",
                self.locks(),
                self.tid()
            );
        }
        let res = crate::kernel::sched::sleep(timeout_ns, flags);

        if self.state() != TaskState::Runnable {
            dbgln!(
                task,
                "task {} state {:?} expected runnable, failing",
                self.tid(),
                self.state()
            );
        }

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

        dbgln!(
            task,
            "signal task: {} signal: {}, this_thread: {}",
            self.tid(),
            sig,
            this_thread
        );

        match self.signals().trigger(sig, this_thread) {
            TriggerResult::Triggered => {
                dbgln!(signal, "signal {} triggered", sig);
                self.wake_up();

                true
            }
            TriggerResult::Execute(f) => {
                dbgln!(signal, "signal {} execute", sig);
                f(sig, self.me());

                true
            }
            TriggerResult::Blocked if !this_thread => {
                dbgln!(signal, "signal {} blocked", sig);
                // Find other thread in process to notify
                let process_leader = self.process_leader();

                if !process_leader.signals().is_blocked(sig) {
                    process_leader.wake_up();

                    return true;
                }

                for c in process_leader
                    .children_debug(8)
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
        dbgln!(
            signal,
            "{}: signal {} sig: {}",
            current_task_ref().tid(),
            self.pid(),
            sig
        );
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

        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(status);
            dbgln!(
                task,
                "add zombie {} to parent {}",
                self.me().tid(),
                parent.tid()
            );
            parent.children_events.add_zombie(self.me());

            if self.is_process_leader() {
                dbgln!(
                    waitpid,
                    "signal zombie SIGCHILD by {} to {}",
                    self.tid(),
                    parent.tid()
                );
                parent.signal(SIGCHLD);
            }
        }
    }

    pub fn notify_continued(&self) {
        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(syscall_defs::waitpid::Status::Continued);
            parent.children_events.add_continued(self.me());

            if self.is_process_leader() {
                dbgln!(waitpid, "signal cont SIGCHILD by {}", self.tid());
                parent.signal(SIGCHLD);
            }
        }
    }

    pub fn notify_stopped(&self, sig: usize) {
        if let Some(parent) = self.get_parent() {
            self.set_waitpid_status(syscall_defs::waitpid::Status::Stopped(sig as u64));
            parent.children_events.add_stopped(self.me());

            if self.is_process_leader() {
                dbgln!(waitpid, "signal stop SIGCHILD by {}", self.tid());
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
        dbgln!(task, "drop task {}", self.tid());
        unsafe {
            dbgln!(task, "deallocate task user {}", self.tid());
            self.arch_task_mut().deallocate_user();
            dbgln!(task, "deallocate task kern {}", self.tid());
            self.arch_task_mut().deallocate_kern();
        }
    }
}
