use crate::kernel::mm::{PhysAddr, VirtAddr};
use crate::kernel::utils::wait_queue::WaitQueue;
use spin::Once;

use crate::kernel::sync::Spin;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use syscall_defs::{SyscallError, SyscallResult};

pub struct Futex {
    lock: Spin<()>,
    wq: WaitQueue,
}

impl Futex {
    fn new() -> Futex {
        Futex {
            lock: Spin::new(()),
            wq: WaitQueue::new(),
        }
    }
}

pub struct FutexContainer {
    fut: Spin<hashbrown::HashMap<PhysAddr, Arc<Futex>>>,
}

impl FutexContainer {
    fn new() -> FutexContainer {
        FutexContainer {
            fut: Spin::new(hashbrown::HashMap::new()),
        }
    }

    pub fn get_alloc(&self, addr: PhysAddr) -> Arc<Futex> {
        let mut futs = self.fut.lock();

        if let Some(f) = futs.get(&addr) {
            f.clone()
        } else {
            let new_f = Arc::new(Futex::new());
            futs.insert(addr, new_f.clone());
            new_f
        }
    }

    pub fn get(&self, addr: PhysAddr) -> Option<Arc<Futex>> {
        let futs = self.fut.lock();

        futs.get(&addr).cloned()
    }

    pub fn wait(&self, addr: VirtAddr, expected: u32) -> SyscallResult {
        if let Some(phys) = addr.to_phys_pagewalk() {
            let futex = self.get_alloc(phys);

            let atom = unsafe { addr.read_ref::<AtomicU32>() };

            let res = futex
                .wq
                .wait_lock_for(&futex.lock, |_l| atom.load(Ordering::SeqCst) == expected);

            if futex.wq.is_empty() {
                self.fut.lock().remove(&phys);
            }

            if let Err(e) = res {
                Err(e.into())
            } else {
                Ok(0)
            }
        } else {
            Err(SyscallError::Inval)
        }
    }

    pub fn wake(&self, addr: VirtAddr) -> SyscallResult {
        if let Some(phys) = addr.to_phys_pagewalk() {
            if let Some(futex) = self.get(phys) {
                futex.wq.notify_all();

                Ok(0)
            } else {
                Err(SyscallError::Inval)
            }
        } else {
            Err(SyscallError::Inval)
        }
    }
}

static FUTEX_CONTAINER: Once<FutexContainer> = Once::new();

pub fn futex() -> &'static FutexContainer {
    FUTEX_CONTAINER.get().unwrap()
}

pub fn init() {
    FUTEX_CONTAINER.call_once(|| FutexContainer::new());
}
