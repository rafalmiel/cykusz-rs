use alloc::sync::Arc;

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

    dead: LinkedList<SchedTaskAdapter>,
    dead_wq: WaitQueue,

    prev_id: usize,
}

impl Default for Queues {
    fn default() -> Queues {
        let idle = Task::this();

        Queues {
            sched_task: Task::new_sched(scheduler_main),
            current: None,
            idle_task: idle,

            runnable: LinkedList::new(SchedTaskAdapter::new()),
            deadline_awaiting: LinkedList::new(SchedTaskAdapter::new()),
            awaiting: LinkedList::new(SchedTaskAdapter::new()),

            dead: LinkedList::new(SchedTaskAdapter::new()),
            dead_wq: WaitQueue::new(),

            prev_id: 0,
        }
    }
}

impl Queues {
    fn switch(&self, to: &Arc<Task>, lock: SpinGuard<()>) {
        set_current(to);

        drop(lock);

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
            let _guard = IrqGuard::new();
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
        self.prev_id = get_current().id();

        self.schedule_check_deadline(&lock);

        if let Some(to_run) = self.runnable.pop_front() {
            if let Some(current) = self.current.clone() {
                if !current.sched.is_linked() && current.id() != to_run.id() {
                    self.push_runnable(current);
                }
            }

            assert_eq!(
                to_run.state(),
                TaskState::Runnable,
                "schedule_next: switching to not runnable task {} {:?}",
                to_run.id(),
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

        self.prev_id != get_current().id()
    }

    fn queue_task(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        //println!("queue task {}", task.id());
        self.push_runnable(task);
    }

    fn sleep(&mut self, time_ns: Option<usize>, lock: SpinGuard<()>) -> SignalResult<()> {
        let task = get_current().clone();

        assert_ne!(task.id(), self.idle_task.id(), "Idle task should not sleep");

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

    fn exit(&mut self, lock: SpinGuard<()>) -> ! {
        let current = self.current.as_ref().unwrap();

        assert_eq!(current.state(), TaskState::Runnable);
        assert_eq!(current.sched.is_linked(), false);

        self.dead.push_back(current.clone());

        self.switch_to_sched(current, lock);

        unreachable!()
    }

    fn reap_dead(&mut self, lock: &mut Spin<()>) {
        let mut locked = lock.lock();
        while let Some(dead) = self.dead.pop_front() {
            dead.set_state(TaskState::Unused);

            drop(locked);
            dead.make_zombie();
            locked = lock.lock();
        }
    }

    fn push_awaiting(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.id(), self.idle_task.id());

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(0);

        self.awaiting.push_back(task);
    }

    fn push_deadline_awaiting(&mut self, task: Arc<Task>, time_ns: usize) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.id(), self.idle_task.id());

        use crate::kernel::timer::current_ns;

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(current_ns() as usize + time_ns);

        self.deadline_awaiting.push_back(task);
    }

    fn push_runnable(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.id(), self.idle_task.id());

        task.set_state(TaskState::Runnable);
        task.set_sleep_until(0);

        self.runnable.push_back(task);
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

    fn exit(&self, _status: isize) -> ! {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.exit(lock);
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

        queue
            .dead_wq
            .wait_lock_for(lock, |_sg| !queue.dead.is_empty())
            .expect("[ SCHED ] Unexpected signal in reaper thread");

        queue.reap_dead(lock);
    }
}

fn reaper() {
    let rr_scheduler = super::scheduler().as_impl::<RRScheduler>();

    loop {
        rr_scheduler.reaper();
    }
}

fn scheduler_main() {
    let rr_scheduler = super::scheduler().as_impl::<RRScheduler>();

    loop {
        rr_scheduler.schedule_next();
    }
}
