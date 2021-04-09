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
            panic!("Task already registered");
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

        if group == 0 || (to.is_none() && group == process.pid()) {
            from.remove_process(process.pid())?;

            if groups.insert(process.pid(), Group::new(process)).is_some() {
                Err(SyscallError::EPERM)
            } else {
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

        let sessions = self.sessions.lock();

        if let Some(session) = sessions.get(&process.sid()) {
            assert!(!process.is_session_leader());

            session.register_task(process);
        } else {
            assert!(process.is_session_leader());

            drop(sessions);

            self.create_session(process);
        }
    }

    pub fn set_sid(&self, process: Arc<Task>) -> SyscallResult {
        if process.is_group_leader() {
            return Err(SyscallError::EPERM);
        }

        let process = process.process_leader();

        self.remove_process(&process)?;

        self.create_session(process);

        Ok(0)
    }

    pub fn set_pgid(&self, pid: usize, gid: usize) -> SyscallResult {
        let caller = current_task().process_leader();

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
