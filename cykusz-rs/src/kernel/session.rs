use alloc::sync::Arc;

use spin::Once;

use syscall_defs::{SyscallError, SyscallResult};

use crate::kernel::sched::{current_task, get_task};
use crate::kernel::sync::Spin;
use crate::kernel::task::Task;

pub struct Group {
    id: usize,
    processes: Spin<hashbrown::HashMap<usize, Arc<Task>>>,
}

impl Group {
    fn new(leader: Arc<Task>) -> Arc<Group> {
        let group = Group {
            id: leader.pid(),
            processes: Spin::new(hashbrown::HashMap::new()),
        };

        leader.set_gid(group.id);

        logln4!("New group {}, sid: {}", leader.pid(), leader.sid());

        group.processes.lock().insert(leader.pid(), leader);

        Arc::new(group)
    }

    fn remove_process(&self, pid: usize) -> SyscallResult {
        let mut procs = self.processes.lock();

        procs
            .remove(&pid)
            .ok_or(SyscallError::ESRCH)
            .map(|_f| procs.len())
    }

    fn add_process(&self, process: Arc<Task>) -> SyscallResult {
        let mut procs = self.processes.lock();

        if procs.insert(process.pid(), process.clone()).is_none() {
            process.set_gid(self.id);

            Ok(0)
        } else {
            Err(SyscallError::EPERM)
        }
    }

    fn register_process(&self, process: Arc<Task>) {
        let mut procs = self.processes.lock();

        if procs.insert(process.pid(), process).is_some() {
            println!("[ SESSION ] Registered task replaced");
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn has_process(&self, pid: usize) -> bool {
        self.processes.lock().contains_key(&pid)
    }

    pub fn signal(&self, sig: usize) {
        let procs = self.processes.lock();

        for (_pid, proc) in procs.iter() {
            proc.signal(sig);
        }
    }

    pub fn for_each(&self, f: &impl Fn(&Arc<Task>)) {
        let procs = self.processes.lock();

        for (_id, task) in procs.iter() {
            f(task);
        }
    }
}

pub struct Session {
    id: usize,
    groups: Spin<hashbrown::HashMap<usize, Arc<Group>>>,
}

impl Session {
    fn new(leader: Arc<Task>) -> Arc<Session> {
        let session = Session {
            id: leader.pid(),
            groups: Spin::new(hashbrown::HashMap::new()),
        };

        logln4!("New session {}", leader.pid());

        leader.set_sid(session.id);

        session
            .groups
            .lock()
            .insert(leader.pid(), Group::new(leader));

        Arc::new(session)
    }

    fn remove_process(&self, process: &Arc<Task>) -> SyscallResult {
        let mut groups = self.groups.lock();

        if let Some(group) = groups.get(&process.gid()) {
            if let Ok(num) = group.remove_process(process.pid()) {
                if num == 0 {
                    logln4!("Remove group {}", process.gid());
                    groups.remove(&process.gid());
                }

                return Ok(groups.len());
            }
        }

        Err(SyscallError::ESRCH)
    }

    fn move_to_group(&self, process: Arc<Task>, group: usize) -> SyscallResult {
        let mut groups = self.groups.lock();

        let from = groups.get(&process.gid()).ok_or(SyscallError::EPERM)?;

        let to = if group != 0 { groups.get(&group) } else { None };

        if group != 0 && to.is_none() {
            for (_id, _g) in groups.iter() {
                logln4!("Group {} not found, existing: {}", group, id);
            }
        }

        logln4!(
            "Move from {} to {}",
            from.id(),
            if to.is_some() { to.unwrap().id() } else { 0 }
        );

        if group == 0 || (to.is_none() && group == process.pid()) {
            from.remove_process(process.pid())?;

            if groups
                .insert(process.pid(), Group::new(process.clone()))
                .is_some()
            {
                Err(SyscallError::EPERM)
            } else {
                logln4!("Group {} inserted", process.gid());
                Ok(0)
            }
        } else if let Some(to) = to {
            from.remove_process(process.pid())?;

            to.add_process(process)
        } else {
            Err(SyscallError::EPERM)
        }
    }

    fn register_task(&self, process: Arc<Task>) {
        let mut groups = self.groups.lock();

        if let Some(group) = groups.get(&process.gid()) {
            assert!(!process.is_group_leader());

            group.register_process(process);
        } else {
            assert!(process.is_group_leader());

            groups.insert(process.gid(), Group::new(process));
        }
    }

    fn get_group(&self, gid: usize) -> Option<Arc<Group>> {
        self.groups.lock().get(&gid).cloned()
    }

    pub fn for_each(&self, f: impl Fn(&Arc<Task>)) {
        let groups = self.groups.lock();

        for (_id, group) in groups.iter() {
            group.for_each(&f);
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub struct Sessions {
    sessions: Spin<hashbrown::HashMap<usize, Arc<Session>>>,
}

impl Sessions {
    fn new() -> Sessions {
        Sessions {
            sessions: Spin::new(hashbrown::HashMap::new()),
        }
    }

    pub fn remove_process(&self, process: &Arc<Task>) -> SyscallResult {
        let mut sessions = self.sessions.lock();

        logln_disabled!("sessions: Remove process: {}", process.tid());

        if let Some(session) = sessions.get(&process.sid()) {
            if let Ok(num) = session.remove_process(process) {
                if num == 0 {
                    sessions.remove(&process.sid());
                }

                return Ok(sessions.len());
            }
        }

        Err(SyscallError::ESRCH)
    }

    fn create_session(&self, process: Arc<Task>) {
        let mut sessions = self.sessions.lock();

        sessions.insert(process.pid(), Session::new(process));
    }

    fn move_to_group(&self, process: Arc<Task>, group: usize) -> SyscallResult {
        let sessions = self.sessions.lock();

        if group == process.gid() || (group == 0 && process.is_group_leader()) {
            return Ok(0);
        }

        let session = sessions.get(&process.sid()).ok_or(SyscallError::ESRCH)?;

        session.move_to_group(process, group)
    }

    pub fn register_process(&self, process: Arc<Task>) {
        if !process.is_process_leader() {
            return;
        }

        logln_disabled!("sessions: Register process: {}", process.tid());

        let sessions = self.sessions.lock();

        if let Some(session) = sessions.get(&process.sid()) {
            assert!(!process.is_session_leader());

            logln4!("Add {} to session {}", process.pid(), session.id());

            session.register_task(process);
        } else {
            assert!(process.is_session_leader());

            drop(sessions);

            self.create_session(process);
        }
    }

    pub fn get_group(&self, sid: usize, gid: usize) -> Option<Arc<Group>> {
        let sessions = self.sessions.lock();

        sessions.get(&sid)?.get_group(gid)
    }

    pub fn get_session(&self, sid: usize) -> Option<Arc<Session>> {
        let sessions = self.sessions.lock();

        sessions.get(&sid).cloned()
    }

    pub fn set_sid(&self, process: Arc<Task>) -> SyscallResult {
        if process.is_group_leader() {
            return Err(SyscallError::EPERM);
        }
        logln3!("setsid {}", process.pid());

        let process = process.process_leader();

        self.remove_process(&process)?;

        self.create_session(process);

        Ok(0)
    }

    pub fn set_pgid(&self, pid: usize, gid: usize) -> SyscallResult {
        let caller = current_task().process_leader();

        logln4!("set_pgid {} {}", pid, gid);

        let process = if pid == 0 || pid == caller.pid() {
            caller.clone()
        } else {
            if let Some(task) = get_task(pid) {
                if !task.is_process_leader() {
                    println!("not a process leader");
                    return Err(SyscallError::EPERM);
                }

                if let Some(parent) = task.get_parent() {
                    if parent.tid() != caller.tid() {
                        println!("not a process child");
                        return Err(SyscallError::ESRCH);
                    }
                } else {
                    println!("not parent");
                    return Err(SyscallError::EPERM);
                }

                task
            } else {
                return Err(SyscallError::ESRCH);
            }
        };

        if process.is_session_leader() {
            println!("session leader");
            return Err(SyscallError::EPERM);
        }

        if caller.sid() != process.sid() {
            println!("different session");
            return Err(SyscallError::EPERM);
        }

        self.move_to_group(process, gid)
    }
}

static SESSIONS: Once<Sessions> = Once::new();

pub fn init() {
    SESSIONS.call_once(|| Sessions::new());
}

pub fn sessions() -> &'static Sessions {
    unsafe { SESSIONS.get_unchecked() }
}
