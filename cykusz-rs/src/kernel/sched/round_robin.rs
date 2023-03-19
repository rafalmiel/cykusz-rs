use alloc::sync::Arc;

use crate::{arch, kernel};
use intrusive_collections::LinkedList;

use crate::kernel::sched::SchedulerInterface;
use crate::kernel::signal::{SignalError, SignalResult};
use crate::kernel::sync::{IrqGuard, Spin, SpinGuard};
use crate::kernel::task::{SchedTaskAdapter, Task, TaskState};
use crate::kernel::utils::wait_queue::WaitQueue;
use crate::kernel::utils::PerCpu;

#[thread_local]
static mut CURRENT_TASK: Option<Arc<Task>> = None;

fn set_current(current: &Arc<Task>) {
    let _guard = IrqGuard::new();

    unsafe {
        CURRENT_TASK = Some(current.clone());
    }
}

fn get_current<'a>() -> &'a Arc<Task> {
    let _guard = IrqGuard::new();

    unsafe { CURRENT_TASK.as_ref().unwrap() }
}

struct Queues {
    sched_task: Arc<Task>,

    current: Option<Arc<Task>>,

    idle_task: Arc<Task>,

    runnable: LinkedList<SchedTaskAdapter>,

    deadline_awaiting: LinkedList<SchedTaskAdapter>,

    awaiting: LinkedList<SchedTaskAdapter>,

    stopped: LinkedList<SchedTaskAdapter>,

    dead: LinkedList<SchedTaskAdapter>,
    dead_wq: WaitQueue,

    prev_id: usize,
}

impl Default for Queues {
    fn default() -> Queues {
        let idle = Task::this();

        logln!("idle task id {}", idle.tid());

        Queues {
            sched_task: Task::new_sched(scheduler_main),
            current: None,
            idle_task: idle,

            runnable: LinkedList::new(SchedTaskAdapter::new()),
            deadline_awaiting: LinkedList::new(SchedTaskAdapter::new()),
            awaiting: LinkedList::new(SchedTaskAdapter::new()),
            stopped: LinkedList::new(SchedTaskAdapter::new()),

            dead: LinkedList::new(SchedTaskAdapter::new()),
            dead_wq: WaitQueue::new(),

            prev_id: 0,
        }
    }
}

impl Queues {
    fn switch(&self, to: &Arc<Task>, lock: SpinGuard<()>) {
        drop(lock);

        set_current(to);

        crate::kernel::sched::finalize();
        unsafe {
            switch!(&self.sched_task, &to);
        }
    }

    fn switch_to_sched(&self, from: &Arc<Task>, lock: SpinGuard<()>) {
        drop(lock);

        if !self.dead.is_empty() {
            self.dead_wq.notify_one();
        }

        unsafe {
            switch!(&from, &self.sched_task);
        }
    }

    fn schedule_check_deadline(&mut self, _lock: &SpinGuard<()>) {
        use crate::kernel::timer::current_ns;

        let time = current_ns() as usize;

        let mut cursor = self.deadline_awaiting.front_mut();

        while let Some(c) = cursor.get() {
            if c.sleep_until() <= time {
                let ptr = cursor.remove().unwrap();

                assert_eq!(ptr.sched.is_linked(), false);
                ptr.set_state(TaskState::Runnable);
                ptr.set_sleep_until(0);

                self.runnable.push_back(ptr);
            } else {
                cursor.move_next();
            }
        }
    }

    fn schedule_next(&mut self, lock: SpinGuard<()>) {
        self.prev_id = get_current().tid();

        self.schedule_check_deadline(&lock);

        if let Some(to_run) = self.runnable.pop_front() {
            if let Some(current) = self.current.clone() {
                //println!("{} -> {}", current.id(), to_run.id());
                if !current.sched.is_linked() && current.tid() != to_run.tid() {
                    self.push_runnable(current);
                }
            }

            assert_eq!(
                to_run.state(),
                TaskState::Runnable,
                "schedule_next: switching to not runnable task {} {:?}",
                to_run.tid(),
                to_run.state()
            );

            self.current = Some(to_run);

            self.switch(&self.current.as_ref().unwrap(), lock);
        } else {
            if let Some(current) = self.current.as_ref() {
                if current.state() == TaskState::Runnable {
                    //println!("self {}", current.id());
                    self.switch(current, lock);

                    return;
                }
            }
            self.current = None;
            self.switch(&self.idle_task, lock);
        }
    }

    fn reschedule(&self, lock: SpinGuard<()>) -> bool {
        self.switch_to_sched(get_current(), lock);

        self.prev_id != get_current().tid()
    }

    fn queue_task(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        //println!("queue task {}", task.id());
        self.push_runnable(task);
    }

    fn sleep(&mut self, time_ns: Option<usize>, lock: SpinGuard<()>) -> SignalResult<()> {
        let task = get_current().clone();

        if task.locks() > 0 {
            logln!(
                "warn: sleeping while holding {} locks, tid: {}",
                task.locks(),
                task.tid()
            );
        }

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        if task.has_pending_io() {
            task.set_has_pending_io(false);
            return Ok(());
        }

        if let Some(time_ns) = time_ns {
            self.push_deadline_awaiting(task, time_ns);
        } else {
            self.push_awaiting(task);
        }

        self.reschedule(lock);

        let task = get_current();

        if task.signals().has_pending() {
            Err(SignalError::Interrupted)
        } else {
            Ok(())
        }
    }

    fn wake(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        if task.state() == TaskState::AwaitingIo {
            let mut cursor = if task.sleep_until() > 0 {
                unsafe { self.deadline_awaiting.cursor_mut_from_ptr(task.as_ref()) }
            } else {
                unsafe { self.awaiting.cursor_mut_from_ptr(task.as_ref()) }
            };

            if let Some(task) = cursor.remove() {
                self.push_runnable(task);
            }
        } else {
            task.set_has_pending_io(true);
        }
    }

    fn wake_as_next(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        if task.state() == TaskState::AwaitingIo {
            let mut cursor = if task.sleep_until() > 0 {
                unsafe { self.deadline_awaiting.cursor_mut_from_ptr(task.as_ref()) }
            } else {
                unsafe { self.awaiting.cursor_mut_from_ptr(task.as_ref()) }
            };

            if let Some(task) = cursor.remove() {
                self.push_runnable_front(task);
            }
        } else {
            task.set_has_pending_io(true);
            let mut cursor = unsafe { self.runnable.cursor_mut_from_ptr(task.as_ref()) };
            if let Some(task) = cursor.remove() {
                self.push_runnable_front(task);
            }
        }
    }

    fn stop(&mut self, lock: SpinGuard<()>) {
        let task = get_current().clone();

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        self.push_stopped(task);

        self.reschedule(lock);
    }

    fn cont(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        if task.state() == TaskState::Stopped {
            let mut cursor = unsafe { self.stopped.cursor_mut_from_ptr(task.as_ref()) };

            if let Some(task) = cursor.remove() {
                self.push_runnable(task);
            }
        }
    }

    fn exit(&mut self, status: isize, lock: SpinGuard<()>) -> ! {
        let current = get_current();

        logln_disabled!(
            "exit tid: {}, sc: {}, wc: {}, st: {}",
            current.tid(),
            Arc::strong_count(current),
            Arc::weak_count(current),
            status
        );

        assert_eq!(current.state(), TaskState::Runnable);
        assert_eq!(current.sched.is_linked(), false);
        assert!(current.is_process_leader());

        self.dead.push_back(current.clone());

        logln!(
            "FREE MEM h:{} p:{}",
            kernel::mm::heap::heap_mem(),
            arch::mm::phys::used_mem()
        );

        current.set_exit_status(status);
        self.switch_to_sched(current, lock);

        unreachable!()
    }

    fn exit_thread(&mut self, _lock: SpinGuard<()>) -> ! {
        let task = get_current();

        assert_eq!(task.state(), TaskState::Runnable);
        assert_eq!(task.sched.is_linked(), false);

        logln_disabled!(
            "exit_thread tid: {}, sc: {}, wc: {}",
            task.tid(),
            Arc::strong_count(task),
            Arc::weak_count(task)
        );

        self.dead.push_back(task.clone());

        self.switch_to_sched(task, _lock);

        unreachable!()
    }

    fn reap_dead(&mut self, locked: SpinGuard<()>) {
        if let Some(dead) = self.dead.pop_front() {
            dead.set_state(TaskState::Unused);

            drop(locked);
            dead.make_zombie();

            logln_disabled!(
                "reap thread zombie {} pl: {} sc: {}, wc: {}",
                dead.tid(),
                dead.is_process_leader(),
                Arc::strong_count(&dead),
                Arc::weak_count(&dead),
            );
        }
    }

    fn push_awaiting(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(0);

        self.awaiting.push_back(task);
    }

    fn push_stopped(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Stopped);
        task.set_sleep_until(0);

        self.stopped.push_back(task);
    }

    fn push_deadline_awaiting(&mut self, task: Arc<Task>, time_ns: usize) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        use crate::kernel::timer::current_ns;

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(current_ns() as usize + time_ns);

        self.deadline_awaiting.push_back(task);
    }

    fn push_runnable(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);

        self.runnable.push_back(task);
    }

    fn push_runnable_front(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);

        self.runnable.push_front(task);
    }
}

pub struct RRScheduler {
    queues: PerCpu<(Spin<()>, Queues)>,
}

unsafe impl Send for RRScheduler {}
unsafe impl Sync for RRScheduler {}

impl SchedulerInterface for RRScheduler {
    fn init(&self) {
        let (_, queue) = self.queues.this_cpu();

        set_current(&queue.idle_task);

        crate::kernel::sched::register_task(&queue.idle_task);

        crate::kernel::sched::create_task(reaper);
    }

    fn reschedule(&self) -> bool {
        let (lock, queue) = self.queues.this_cpu();

        let lock = lock.lock_irq();

        queue.reschedule(lock)
    }

    fn current_task<'a>(&self) -> &'a Arc<Task> {
        get_current()
    }

    fn queue_task(&self, task: Arc<Task>) {
        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.queue_task(task, lock);
    }

    fn sleep(&self, until: Option<usize>) -> SignalResult<()> {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.sleep(until, lock)
    }

    fn wake(&self, task: Arc<Task>) {
        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.wake(task, lock);
    }

    fn wake_as_next(&self, task: Arc<Task>) {
        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.wake_as_next(task, lock);
    }

    fn cont(&self, task: Arc<Task>) {
        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.cont(task, lock);
    }

    fn stop(&self) {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.stop(lock);
    }

    fn exit(&self, status: isize) -> ! {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.exit(status, lock);
    }

    fn exit_thread(&self) -> ! {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.exit_thread(lock);
    }
}

impl RRScheduler {
    pub fn new() -> Arc<RRScheduler> {
        Arc::new(RRScheduler {
            queues: PerCpu::new_fn(|| (Spin::new(()), Queues::default())),
        })
    }

    fn schedule_next(&self) {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.schedule_next(lock);
    }

    fn reaper(&self) {
        let (lock, queue) = self.queues.this_cpu_mut();

        loop {
            let locked = queue
                .dead_wq
                .wait_lock_irq_for(lock, |_sg| {
                    let _ = &queue;
                    !queue.dead.is_empty()
                })
                .expect("[ SCHED ] Unexpected signal in reaper thread");

            queue.reap_dead(locked);
        }
    }
}

fn reaper() {
    let rr_scheduler = super::scheduler().as_impl::<RRScheduler>();

    rr_scheduler.reaper();
}

fn scheduler_main() {
    let rr_scheduler = super::scheduler().as_impl::<RRScheduler>();

    loop {
        rr_scheduler.schedule_next();
    }
}
