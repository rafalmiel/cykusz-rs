use alloc::sync::Arc;
use bit_field::BitField;

use intrusive_collections::LinkedList;
use syscall_defs::{SyscallError, SyscallResult};
use syscall_defs::waitpid::WaitPidFlags;
use crate::kernel::sched::current_task_ref;

use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::task::{WaitPidTaskAdapter, Task, TaskState};
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(Default)]
struct TaskLists {
    tasks: LinkedList<WaitPidTaskAdapter>,
}

#[derive(Default)]
pub struct WaitPidEvents {
    tasks: Spin<TaskLists>,
    wq: WaitQueue,
}

impl WaitPidEvents {
    pub fn add_zombie(&self, zombie: Arc<Task>) {
        assert_eq!(zombie.sched.is_linked(), false);
        assert_eq!(zombie.state(), TaskState::Unused);

        let mut list = self.tasks.lock();

        logln2!("add zombie pid {}", zombie.pid());

        list.tasks.push_back(zombie);

        self.wq.notify_one();
    }

    pub fn add_stopped(&self, stopped: Arc<Task>) {
        assert_eq!(stopped.state(), TaskState::Stopped);

        let mut list = self.tasks.lock();

        logln2!("{} add stopcont pid {}", current_task_ref().pid(), stopped.pid());

        list.tasks.push_back(stopped);

        drop(list);

        self.wq.notify_one();
    }

    pub fn add_continued(&self, continued: Arc<Task>) {
        //assert_eq!(continued.sched.is_linked(), true);

        if !continued.waitpid.is_linked() {
            let mut list = self.tasks.lock();

            logln2!("add stopcont pid {}", continued.pid());

            list.tasks.push_back(continued);
        }

        self.wq.notify_one();
    }

    fn wait_on(&self, flags: WaitPidFlags, cond: impl Fn(&Task) -> bool) -> SignalResult<Option<(usize, syscall_defs::waitpid::Status)>> {
        let mut res = (0, syscall_defs::waitpid::Status::Invalid(0));

        let result = self.wq.wait_lock_for_no_hang(flags.nohang(), &self.tasks, |l| {
            if flags.exited() {
                let mut cur = l.tasks.front_mut();

                while let Some(t) = cur.get() {
                    logln2!("checking task {}", t.pid());
                    if cond(t) {
                        res = (t.tid(), t.waitpid_status());

                        cur.remove();

                        return true;
                    } else {
                        cur.move_next();
                    }
                }
            }

            if flags.stopped() || flags.continued() {
                let mut cur = l.tasks.front_mut();

                while let Some(t) = cur.get() {
                    logln2!("checking stopcont task {}", t.pid());
                    let status = t.waitpid_status();
                    if ((flags.continued() && status.is_continued()) || (flags.stopped() && status.is_stopped()))
                        && cond(t) {

                        res = (t.tid(), status);

                        cur.remove();

                        return true;
                    } else {
                        cur.move_next();
                    }
                }

            }

            false
        })?;

        if result.is_some() {
            logln2!("WAIT RESULT {:?}", res);
            return Ok(Some(res));
        }

        return Ok(None);
    }

    pub fn wait_thread(&self, pid: usize, tid: usize) -> SignalResult<usize> {
        self.wait_on(WaitPidFlags::empty(), |t| pid == t.pid() && (t.tid() == tid || tid == 0))
            .map(|res| {
                if let Some((tid, status)) = res {
                    tid
                } else {
                    panic!("unexpected no thread wait");
                }
            })
    }

    pub fn wait_pid(&self, pid: isize, status: &mut syscall_defs::waitpid::Status, flags: WaitPidFlags) -> SignalResult<SyscallResult> {
        logln2!("{} wait on {} {:?}", current_task_ref().pid(), pid, flags);

        let ret = self.wait_on(flags, |t| {
            logln2!("got wait on task {} {} {}", t.is_process_leader(), t.gid(), t.pid());
            if !t.is_process_leader() {
                false
            } else {
                match pid {
                    g if pid < -1 => {
                        logln2!("here1");
                        t.gid() == (-g) as usize
                    },
                    -1 => {
                        logln2!("here2");
                        true
                    },
                    0 => {
                        logln2!("here3");
                        current_task_ref().gid() == t.gid()
                    },
                    p => {
                        logln2!("here4");
                        t.pid() == p as usize
                    }
                }
            }
        });
        logln2!("wait {} finished", pid);

        return match ret {
            Ok(Some((tid, st))) => {
                *status = st;

                Ok(Ok(tid))
            },
            Ok(None) => {
                Ok(Err(SyscallError::ECHILD))
            }
            Err(e) => Err(e),
        };
    }
}
