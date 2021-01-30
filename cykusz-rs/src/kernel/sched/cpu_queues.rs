use alloc::sync::Arc;
use core::cell::UnsafeCell;

use crate::kernel::sync::Spin;
use crate::kernel::task::Task;
use crate::kernel::utils::PerCpu;

use super::CpuQueue;

pub struct CpuQueues {
    cpu_queues_locks: PerCpu<Spin<()>>,
    cpu_queues: PerCpu<UnsafeCell<CpuQueue>>,
}

unsafe impl Sync for CpuQueues {}

impl Default for CpuQueues {
    fn default() -> CpuQueues {
        CpuQueues {
            cpu_queues_locks: PerCpu::new_fn(|| Spin::<()>::new(())),
            cpu_queues: PerCpu::new_fn(|| UnsafeCell::new(CpuQueue::default())),
        }
    }
}

impl CpuQueues {
    unsafe fn this_cpu_queue(&self) -> &mut CpuQueue {
        &mut *(self.cpu_queues.this_cpu_mut().get())
    }

    unsafe fn cpu_queue(&self, cpu: isize) -> &mut CpuQueue {
        &mut *(self.cpu_queues.cpu_mut(cpu).get())
    }

    pub fn schedule_next(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().schedule_next(mutex);
        }
    }

    pub fn reschedule(&self) -> bool {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe { self.this_cpu_queue().reschedule(mutex) }
    }

    pub fn activate_sched(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().activate_sched(mutex);
        }
    }

    pub fn enter_critical_section(&self) {
        let _mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().enter_critical_section();
        }
    }

    pub fn leave_critical_section(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().leave_critical_section(mutex);
        }
    }

    pub fn add_task(&self, task: Arc<Task>) {
        let _mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().add_task(task);
        }
    }

    pub fn current_task_finished(&self) -> ! {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe { self.this_cpu_queue().current_task_finished(mutex) }
    }

    pub fn current_task(&self) -> Arc<Task> {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe { self.this_cpu_queue().current_task(mutex).clone() }
    }

    pub fn init_tasks(&self) {
        for i in 0..crate::kernel::smp::cpu_count() {
            unsafe { self.cpu_queue(i as isize).register_main_task() }
        }
    }
}
