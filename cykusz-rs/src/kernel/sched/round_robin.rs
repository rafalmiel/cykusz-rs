use alloc::sync::Arc;

use crate::arch::int;
use intrusive_collections::LinkedList;

use crate::kernel::sched::{SchedulerInterface, SleepFlags};
use crate::kernel::signal::{SignalError, SignalResult};
use crate::kernel::sync::{IrqGuard, Spin, SpinGuard};
use crate::kernel::task::{SchedTaskAdapter, Task, TaskState};

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

            prev_id: 0,
        }
    }
}

impl Queues {
    fn switch(&self, to: &Arc<Task>, lock: SpinGuard<()>) {
        let _irq = lock.to_irq_guard();

        set_current(to);

        crate::kernel::sched::finalize();
        unsafe {
            switch!(&self.sched_task, &to);
        }
    }

    fn switch_to_sched(&self, from: &Arc<Task>, lock: SpinGuard<()>) {
        let _irq = lock.to_irq_guard();

        if int::is_enabled() {
            panic!("INT ENABLED");
        }

        unsafe {
            switch!(&from, &self.sched_task);
        }
    }

    fn switch_to_sched_exec<F: FnOnce()>(&self, from: &Arc<Task>, lock: SpinGuard<()>, fun: F) {
        let _irq = lock.to_irq_guard();

        if int::is_enabled() {
            panic!("INT ENABLED");
        }

        fun();

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

    #[allow(dead_code)]
    fn debug_sched(&self) {
        if let Some(t) = &self.current {
            log!("c: {}| ", t.tid());
        } else {
            log!("c: None| ");
        }

        for t in &self.runnable {
            log!("{} ", t.tid());
        }
    }

    fn schedule_next(&mut self, lock: SpinGuard<()>) {
        self.prev_id = get_current().tid();

        self.schedule_check_deadline(&lock);

        //self.debug_sched();

        if let Some(to_run) = self.runnable.pop_front() {
            if let Some(current) = self.current.clone() {
                //println!("{} -> {}", current.id(), to_run.id());
                if !current.sched.is_linked()
                    && current.state() == TaskState::Runnable
                    && current.tid() != to_run.tid()
                {
                    self.push_runnable(current, false);
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
    fn reschedule_exec<F: FnOnce()>(&self, lock: SpinGuard<()>, fun: F) -> bool {
        self.switch_to_sched_exec(get_current(), lock, fun);

        self.prev_id != get_current().tid()
    }

    fn queue_task(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        //println!("queue task {}", task.id());
        self.push_runnable(task, false);
    }

    fn sleep(
        &mut self,
        time_ns: Option<usize>,
        flags: SleepFlags,
        lock: SpinGuard<()>,
    ) -> SignalResult<()> {
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

        if !flags.contains(SleepFlags::NON_INTERRUPTIBLE) && task.signals().has_pending() {
            return Err(SignalError::Interrupted);
        }

        let pending = task.signals().pending();

        if pending > 0 {
            logln!("WARN sleep with pending signals: {:#x}", pending);
        }

        // TODO: mark task as uninterruptible and dont wake it up on signals
        if let Some(time_ns) = time_ns {
            self.push_deadline_awaiting(task, time_ns);
        } else {
            self.push_awaiting(task);
        }

        self.reschedule(lock);

        let task = get_current();

        if task.signals().pending() != pending {
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
                self.push_runnable(task, false);
            }
        } else {
            if task.state() == TaskState::Unused {
                panic!("WARN: set pending io on UNUSED task");
            }
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
                self.push_runnable_front(task, false);
            }
        } else if task.state() == TaskState::Runnable {
            task.set_has_pending_io(true);
            let mut cursor = unsafe { self.runnable.cursor_mut_from_ptr(task.as_ref()) };
            if let Some(task) = cursor.remove() {
                self.push_runnable_front(task, false);
            }
        } else {
            task.set_has_pending_io(true);
            //let mut cursor = unsafe { self.stopped.cursor_mut_from_ptr(task.as_ref()) };
            //if let Some(task) = cursor.remove() {
            //    self.push_runnable_front(task, false);
            //}
        }
    }

    fn stop(&mut self, sig: usize, lock: SpinGuard<()>) {
        let task = get_current().clone();

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        self.push_stopped(sig, task.clone());

        self.reschedule_exec(lock, || {
            task.notify_stopped(sig);
        });
    }

    fn cont(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        if task.state() == TaskState::Stopped {
            assert!(task.sched.is_linked());
            let mut cursor = unsafe { self.stopped.cursor_mut_from_ptr(task.as_ref()) };

            if let Some(task) = cursor.remove() {
                self.push_runnable_front(task, true);
            }
        }
    }

    fn exit(&mut self, status: syscall_defs::waitpid::Status, lock: SpinGuard<()>) -> ! {
        let current = get_current();

        logln!(
            "exit tid: {}, sc: {}, wc: {}, st: {:?}",
            current.tid(),
            Arc::strong_count(current),
            Arc::weak_count(current),
            status,
        );

        assert_eq!(current.state(), TaskState::Runnable);
        assert_eq!(current.sched.is_linked(), false);
        assert!(current.is_process_leader());

        logln!(
            "FREE MEM h:{} p:{}",
            crate::kernel::mm::heap::heap_mem(),
            crate::arch::mm::phys::used_mem()
        );

        current.set_state(TaskState::Unused);
        assert_eq!(current.sched.is_linked(), false);

        self.switch_to_sched_exec(current, lock, || {
            current.make_zombie(status);
        });

        unreachable!()
    }

    fn exit_thread(&mut self, _lock: SpinGuard<()>) -> ! {
        let task = get_current();

        assert_eq!(task.state(), TaskState::Runnable);
        assert_eq!(task.sched.is_linked(), false);

        logln!(
            "exit_thread tid: {}, sc: {}, wc: {}",
            task.tid(),
            Arc::strong_count(task),
            Arc::weak_count(task),
        );

        task.set_state(TaskState::Unused);
        self.switch_to_sched_exec(task, _lock, || {
            task.make_zombie(syscall_defs::waitpid::Status::Exited(0));
        });

        logln!("UNEXPECTED EXIT THREAD TID {}", task.tid());

        unreachable!()
    }

    fn push_awaiting(&mut self, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(0);

        self.awaiting.push_back(task);
    }

    fn push_stopped(&mut self, _sig: usize, task: Arc<Task>) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Stopped);
        //task.set_sleep_until(0);

        self.stopped.push_back(task.clone());
    }

    fn push_deadline_awaiting(&mut self, task: Arc<Task>, time_ns: usize) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        use crate::kernel::timer::current_ns;

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(current_ns() as usize + time_ns);

        self.deadline_awaiting.push_back(task);
    }

    fn push_runnable(&mut self, task: Arc<Task>, continued: bool) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);
        self.runnable.push_back(task.clone());

        if continued {
            task.set_has_pending_io(true);
        }
    }

    fn push_runnable_front(&mut self, task: Arc<Task>, continued: bool) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);

        self.runnable.push_front(task.clone());

        if continued {
            task.set_has_pending_io(true);
        }
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

    fn sleep(&self, until: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.sleep(until, flags, lock)
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

    fn stop(&self, sig: usize) {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.stop(sig, lock);
    }

    fn exit(&self, status: syscall_defs::waitpid::Status) -> ! {
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
}

fn scheduler_main() {
    let rr_scheduler = super::scheduler().as_impl::<RRScheduler>();

    loop {
        rr_scheduler.schedule_next();
    }
}
