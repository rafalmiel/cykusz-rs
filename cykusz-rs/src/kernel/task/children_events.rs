use intrusive_collections::LinkedList;

use syscall_defs::waitpid::WaitPidFlags;
use syscall_defs::{SyscallError, SyscallResult};

use crate::kernel::sched::current_task_ref;
use crate::kernel::session::sessions;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::task::{ArcTask, Task, WaitPidTaskAdapter};
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

#[derive(Default)]
struct TaskLists {
    tasks: LinkedList<WaitPidTaskAdapter>,
}

#[derive(Default)]
pub struct WaitPidEvents {
    tasks: Spin<TaskLists>,
    wq: WaitQueue,
    wq_threads: WaitQueue,
}

impl WaitPidEvents {
    pub fn migrate(&self, other: &WaitPidEvents) {
        {
            let mut this_tasks = self.tasks.lock_irq();
            let mut other_tasks = other.tasks.lock_irq();

            dbgln!(
                task,
                "migrate waitpid len: {}",
                other_tasks.tasks.iter().count()
            );

            while let Some(t) = other_tasks.tasks.pop_front() {
                dbgln!(task, "migrate task {}", t.tid());
                this_tasks.tasks.push_back(t);
            }
        }

        self.wq.notify_all();
    }

    pub fn add_zombie(&self, zombie: ArcTask) {
        let is_process = zombie.is_process_leader();

        if !zombie.waitpid.is_linked() {
            let mut list = self.tasks.lock_irq();

            list.tasks.push_back(zombie);
        }

        if is_process {
            dbgln!(notify, "wq notify all");
            self.wq.notify_all_debug();
        } else {
            dbgln!(notify, "wq_threads notify all");
            self.wq_threads.notify_all_debug();
        }
    }

    pub fn add_stopped(&self, stopped: ArcTask) {
        let is_process = stopped.is_process_leader();

        if !stopped.waitpid.is_linked() {
            let mut list = self.tasks.lock_irq();

            list.tasks.push_back(stopped);
        }

        if is_process {
            self.wq.notify_all_debug();
        } else {
            self.wq_threads.notify_all_debug();
        }
    }

    pub fn add_continued(&self, continued: ArcTask) {
        let is_process = continued.is_process_leader();

        if !continued.waitpid.is_linked() {
            let mut list = self.tasks.lock_irq();

            list.tasks.push_back(continued);
        }

        if is_process {
            self.wq.notify_all_debug();
        } else {
            self.wq_threads.notify_all_debug();
        }
    }

    fn wait_on(
        &self,
        wq: &WaitQueue,
        me: &Task,
        flags: WaitPidFlags,
        no_intr: bool,
        mut cond: impl FnMut(&Task) -> bool,
    ) -> SignalResult<Option<(usize, syscall_defs::waitpid::Status)>> {
        let mut res = (0, syscall_defs::waitpid::Status::Invalid(0), None);

        let mut wq_flags: WaitQueueFlags = if flags.nohang() {
            WaitQueueFlags::NO_HANG
        } else {
            WaitQueueFlags::IRQ_DISABLE
        };

        if no_intr {
            wq_flags.insert(WaitQueueFlags::NON_INTERRUPTIBLE);
        }

        let found = wq
            .wait_lock_for(wq_flags, &self.tasks, |l| {
                let mut cur = l.tasks.front_mut();

                while let Some(t) = cur.get() {
                    let status = t.waitpid_status();
                    dbgln!(waitpid, "checking task {} = {:?}", t.tid(), status);
                    if ((flags.exited() && (status.is_exited() || status.is_signaled()))
                        || (flags.continued() && status.is_continued())
                        || (flags.stopped() && status.is_stopped()))
                        && cond(t)
                    {
                        res = (t.tid(), status, Some(t.me()));

                        dbgln!(
                            task,
                            "task {} waitpid remove {} {} {}",
                            t.tid(),
                            t.sibling.is_linked(),
                            t.sched.is_linked(),
                            t.waitpid.is_linked()
                        );

                        cur.remove();

                        return true;
                    } else {
                        cur.move_next();
                    }
                }

                !me.children_debug(2).iter().any(&mut cond)
            })?
            .is_some();

        if found && !res.1.is_invalid() {
            if res.1.is_exited() || res.1.is_signaled() {
                let task = res.2.unwrap();
                task.remove_from_parent();

                if task.is_process_leader() {
                    if let Err(e) = sessions().remove_process(&task) {
                        panic!("Failed to remove process from a session {:?}", e);
                    }
                    dbgln!(
                        task,
                        "task {} remove process from sessions sc: {} wc: {}",
                        task.tid(),
                        ArcTask::strong_count(&task),
                        ArcTask::weak_count(&task)
                    );
                }

                dbgln!(
                    task,
                    "task {} wait_on sc: {} wc: {}",
                    task.tid(),
                    ArcTask::strong_count(&task),
                    ArcTask::weak_count(&task)
                );
            }

            return Ok(Some((res.0, res.1)));
        }

        Ok(None)
    }

    pub fn wait_thread(
        &self,
        me: &Task,
        pid: usize,
        tid: usize,
        flags: WaitPidFlags,
        no_intr: bool,
    ) -> SignalResult<SyscallResult> {
        dbgln!(
            waitpid,
            "task {} wait_thread pid {} tid {} flags {:?} no_intr: {}",
            me.tid(),
            pid,
            tid,
            flags,
            no_intr
        );
        let res = self.wait_on(&self.wq_threads, me, flags, no_intr, |t| {
            dbgln!(
                waitpid,
                "task {} wait_thread check run, candidate: {}",
                me.tid(),
                t.tid()
            );
            !t.is_process_leader() && pid == t.pid() && (t.tid() == tid || tid == 0)
        });

        match res {
            Ok(Some((tid, _st))) => Ok(Ok(tid)),
            Ok(None) => Ok(Err(SyscallError::ECHILD)),
            Err(e) => Err(e),
        }
    }

    pub fn wait_pid(
        &self,
        me: &Task,
        pid: isize,
        status: &mut syscall_defs::waitpid::Status,
        flags: WaitPidFlags,
    ) -> SignalResult<SyscallResult> {
        dbgln!(waitpid, "task {} waitpid {} {:?}", me.tid(), pid, flags);
        let ret = self.wait_on(&self.wq, me, flags, false, |t| {
            dbgln!(
                waitpid,
                "task {} wait_pid check run, candidate: {}",
                me.tid(),
                t.tid()
            );
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
        dbgln!(
            waitpid,
            "{} waitpid {} {:?} = {:?}",
            me.tid(),
            pid,
            flags,
            ret
        );

        match ret {
            Ok(Some((tid, st))) => {
                *status = st;

                Ok(Ok(tid))
            }
            Ok(None) => Ok(Err(SyscallError::ECHILD)),
            Err(e) => Err(e),
        }
    }
}
