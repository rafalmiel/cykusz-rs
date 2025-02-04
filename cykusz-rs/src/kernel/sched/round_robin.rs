use crate::kernel::sched::SchedulerInterface;
use crate::kernel::sync::{IrqGuard, LockApi, Spin, SpinGuard};
use crate::kernel::task::{SchedTaskAdapter, Task, TaskState};
use crate::kernel::utils::PerCpu;
use alloc::sync::Arc;
use intrusive_collections::LinkedList;

#[thread_local]
static mut CURRENT_TASK: Option<Arc<Task>> = None;

fn set_current(current: &Arc<Task>) {
    let _guard = IrqGuard::new();

    unsafe {
        let t = &raw mut CURRENT_TASK;
        let _prev = t.read(); //run destructor of previous entry
        t.write(Some(current.clone()));
    }
}

fn get_current<'a>() -> &'a Arc<Task> {
    let _guard = IrqGuard::new();

    unsafe {
        let t = &raw const CURRENT_TASK;
        t.as_ref_unchecked().as_ref().unwrap()
    }
}

struct Queues {
    idle_task: Arc<Task>,

    runnable: LinkedList<SchedTaskAdapter>,

    deadline_awaiting: LinkedList<SchedTaskAdapter>,

    awaiting: LinkedList<SchedTaskAdapter>,

    stopped: LinkedList<SchedTaskAdapter>,
}

impl Default for Queues {
    fn default() -> Queues {
        let idle = Task::this();

        dbgln!(sched, "idle task: {}", idle.tid());

        Queues {
            idle_task: idle,

            runnable: LinkedList::new(SchedTaskAdapter::new()),
            deadline_awaiting: LinkedList::new(SchedTaskAdapter::new()),
            awaiting: LinkedList::new(SchedTaskAdapter::new()),
            stopped: LinkedList::new(SchedTaskAdapter::new()),
        }
    }
}

impl Queues {
    fn switch<F: FnOnce()>(&self, to: &Arc<Task>, lock: SpinGuard<()>, f: Option<F>) {
        let _irq = lock.to_irq_guard();

        if let Some(f) = f {
            f();
        }

        let prev = get_current().clone();

        set_current(to);

        crate::kernel::sched::finalize();

        dbgln!(sched, "{} -> {}", prev.tid(), to.tid());

        if prev.tid() == to.tid() {
            return;
        }

        unsafe {
            switch!(prev, to);
        }
    }

    fn schedule_check_deadline(&mut self, _lock: &SpinGuard<()>) {
        if self.deadline_awaiting.is_empty() {
            return;
        }

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
        log!("c: {}| ", get_current().tid());

        for t in &self.runnable {
            log!("{} ", t.tid());
        }
    }

    fn schedule_next<F: FnOnce()>(&mut self, lock: SpinGuard<()>, f: Option<F>) -> bool {
        self.schedule_check_deadline(&lock);

        let current = get_current().clone();

        let prev_id = current.tid();

        let to_run = self.runnable.pop_front().unwrap_or_else(|| {
            if current.tid() != self.idle_task.tid() && current.state() == TaskState::Runnable {
                current.clone()
            } else {
                self.idle_task.clone()
            }
        });

        if current.tid() != to_run.tid()
            && !current.sched.is_linked()
            && current.state() == TaskState::Runnable
        {
            self.push_runnable(current, false);
        }

        self.switch(&to_run, lock, f);

        prev_id != to_run.tid()
    }

    fn reschedule(&mut self, lock: SpinGuard<()>) -> bool {
        self.schedule_next(lock, Option::<fn()>::None)
    }

    fn reschedule_exec<F: FnOnce()>(&mut self, lock: SpinGuard<()>, fun: F) -> bool {
        self.schedule_next(lock, Some(fun))
    }

    fn queue_task(&mut self, task: Arc<Task>, _lock: SpinGuard<()>) {
        //println!("queue task {}", task.id());
        self.push_runnable(task, false);
    }

    fn sleep(&mut self, time_ns: Option<usize>, lock: SpinGuard<()>) {
        let task = get_current().clone();

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        // TODO: mark task as uninterruptible and dont wake it up on signals
        if let Some(time_ns) = time_ns {
            dbgln!(task, "task {} pushed deadline awaiting", task.tid());
            self.push_deadline_awaiting(task, time_ns);
        } else {
            dbgln!(task, "task {} pushed awaiting", task.tid());
            self.push_awaiting(task);
        }

        self.reschedule(lock);
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
        } else {
            // if process was not stopped, wake it up in case it has some pending signals to process
            self.wake(task, _lock)
        }
    }

    fn exit(&mut self, status: syscall_defs::waitpid::Status, lock: SpinGuard<()>) -> ! {
        let current = get_current();

        dbgln!(
            task,
            "exit tid: {}, sc: {}, wc: {}, st: {:?}",
            current.tid(),
            Arc::strong_count(current),
            Arc::weak_count(current),
            status,
        );

        assert_eq!(current.state(), TaskState::Unused);
        assert_eq!(current.sched.is_linked(), false);
        assert!(current.is_process_leader());

        dbgln!(
            mem,
            "FREE MEM h:{} p:{}",
            crate::kernel::mm::heap::heap_mem(),
            crate::arch::mm::phys::used_mem()
        );

        self.reschedule_exec(lock, || {
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

        self.reschedule_exec(_lock, || {
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

    fn push_runnable(&mut self, task: Arc<Task>, _continued: bool) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);
        self.runnable.push_back(task.clone());
    }

    fn push_runnable_front(&mut self, task: Arc<Task>, _continued: bool) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);
        self.runnable.push_front(task.clone());
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
        let (lock, queue) = self.queues.this_cpu_mut();

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

    fn sleep(&self, until: Option<usize>) {
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
}
