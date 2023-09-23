use alloc::sync::Arc;

use crate::kernel::sched::current_task_ref;
use intrusive_collections::LinkedList;
use syscall_defs::waitpid::WaitPidFlags;
use syscall_defs::{SyscallError, SyscallResult};

use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::task::{Task, WaitPidTaskAdapter};
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

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
        if !zombie.waitpid.is_linked() {
            let mut list = self.tasks.lock();

            list.tasks.push_back(zombie);
        }

        self.wq.notify_all();
    }

    pub fn add_stopped(&self, stopped: Arc<Task>) {
        if !stopped.waitpid.is_linked() {
            let mut list = self.tasks.lock();

            list.tasks.push_back(stopped);
        }

        self.wq.notify_all();
    }

    pub fn add_continued(&self, continued: Arc<Task>) {
        if !continued.waitpid.is_linked() {
            let mut list = self.tasks.lock();

            list.tasks.push_back(continued);
        }

        self.wq.notify_all();
    }

    fn wait_on(
        &self,
        me: &Task,
        flags: WaitPidFlags,
        no_intr: bool,
        mut cond: impl FnMut(&Task) -> bool,
    ) -> SignalResult<Option<(usize, syscall_defs::waitpid::Status)>> {
        let mut res = (0, syscall_defs::waitpid::Status::Invalid(0));

        let mut wq_flags: WaitQueueFlags = if flags.nohang() {
            WaitQueueFlags::NO_HANG
        } else {
            WaitQueueFlags::empty()
        };

        if no_intr {
            wq_flags.insert(WaitQueueFlags::NON_INTERRUPTIBLE);
        }

        let result = self.wq.wait_lock_for(wq_flags, &self.tasks, |l| {
            let mut cur = l.tasks.front_mut();

            while let Some(t) = cur.get() {
                let status = t.waitpid_status();
                if ((flags.exited() && (status.is_exited() || status.is_signaled()))
                    || (flags.continued() && status.is_continued())
                    || (flags.stopped() && status.is_stopped()))
                    && cond(t)
                {
                    res = (t.tid(), status);

                    if status.is_exited() || status.is_signaled() {
                        t.remove_from_parent();
                    }

                    cur.remove();

                    return true;
                } else {
                    cur.move_next();
                }
            }

            !me.children().iter().any(&mut cond)
        })?;

        if result.is_some() && !res.1.is_invalid() {
            return Ok(Some(res));
        }

        return Ok(None);
    }

    pub fn wait_thread(
        &self,
        me: &Task,
        pid: usize,
        tid: usize,
        flags: syscall_defs::waitpid::WaitPidFlags,
        no_intr: bool,
    ) -> SignalResult<SyscallResult> {
        let res = self.wait_on(me, flags, no_intr, |t| {
            !t.is_process_leader() && pid == t.pid() && (t.tid() == tid || tid == 0)
        });

        return match res {
            Ok(Some((tid, _st))) => Ok(Ok(tid)),
            Ok(None) => Ok(Err(SyscallError::ECHILD)),
            Err(e) => Err(e),
        };
    }

    pub fn wait_pid(
        &self,
        me: &Task,
        pid: isize,
        status: &mut syscall_defs::waitpid::Status,
        flags: WaitPidFlags,
    ) -> SignalResult<SyscallResult> {
        let ret = self.wait_on(me, flags, false, |t| {
            if !t.is_process_leader() {
                false
            } else {
                match pid {
                    g if pid < -1 => t.gid() == (-g) as usize,
                    -1 => true,
                    0 => current_task_ref().gid() == t.gid(),
                    p => t.pid() == p as usize,
                }
            }
        });

        return match ret {
            Ok(Some((tid, st))) => {
                *status = st;

                Ok(Ok(tid))
            }
            Ok(None) => Ok(Err(SyscallError::ECHILD)),
            Err(e) => Err(e),
        };
    }
}
