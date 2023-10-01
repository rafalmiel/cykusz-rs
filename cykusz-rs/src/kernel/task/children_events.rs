use alloc::sync::Arc;
use bit_field::BitField;

use intrusive_collections::LinkedList;
use syscall_defs::{SyscallError, SyscallResult};
use crate::kernel::sched::current_task_ref;

use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::task::{SchedTaskAdapter, Task, TaskState};
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(Default)]
pub struct ChildrenEvents {
    zombies: Spin<LinkedList<SchedTaskAdapter>>,
    zombies_wq: WaitQueue,
}

impl ChildrenEvents {
    pub fn add_zombie(&self, zombie: Arc<Task>) {
        assert_eq!(zombie.sched.is_linked(), false);
        assert_eq!(zombie.state(), TaskState::Unused);

        let mut list = self.zombies.lock();

        logln2!("add zombie pid {}", zombie.pid());

        list.push_back(zombie);

        self.zombies_wq.notify_one();
    }

    fn wait_on(&self, nohang: bool, cond: impl Fn(&Task) -> bool) -> SignalResult<Option<(usize, syscall_defs::waitpid::Status)>> {
        let mut res = (0, syscall_defs::waitpid::Status::Invalid(0));
        let result = self.zombies_wq.wait_lock_for_no_hang(nohang, &self.zombies, |l| {
            let mut cur = l.front_mut();

            while let Some(t) = cur.get() {
                if cond(t) {
                    res = (t.tid(), t.exit_status());

                    cur.remove();

                    return true;
                } else {
                    cur.move_next();
                }
            }

            false
        })?;

        if result.is_none() {
            return Ok(None);
        }

        Ok(Some(res))
    }

    pub fn wait_thread(&self, pid: usize, tid: usize) -> SignalResult<usize> {
        self.wait_on(false, |t| pid == t.pid() && (t.tid() == tid || tid == 0))
            .map(|res| {
                if let Some((tid, status)) = res {
                    tid
                } else {
                    panic!("unexpected no thread wait");
                }
            })
    }

    pub fn wait_pid(&self, pid: isize, status: &mut syscall_defs::waitpid::Status, flags: u64) -> SignalResult<SyscallResult> {
        logln2!("wait on {}", pid);
        let ret = self.wait_on(flags.get_bit(0), |t| {
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
