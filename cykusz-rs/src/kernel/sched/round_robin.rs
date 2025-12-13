use crate::kernel::sched::{SchedulerInterface, SleepFlags};
use crate::kernel::signal::{SignalError, SignalResult};
use crate::kernel::sync::{IrqGuard, LockApi, Spin, SpinGuard};
use crate::kernel::task::{ArcTask, SchedTaskAdapter, Task, TaskState};
use crate::kernel::utils::PerCpu;
use alloc::sync::Arc;
use intrusive_collections::LinkedList;

#[thread_local]
static mut CURRENT_TASK: Option<ArcTask> = None;

fn set_current(current: ArcTask) {
    let _guard = IrqGuard::new();

    unsafe {
        let t = &raw mut CURRENT_TASK;
        drop(t.read()); //run destructor of previous entry
        t.write(Some(current));
    }
}

fn get_current<'a>() -> &'a Task {
    let _guard = IrqGuard::new();

    unsafe {
        let t = &raw const CURRENT_TASK;
        t.as_ref().unwrap().as_deref().unwrap()
    }
}

struct Queues {
    idle_task: ArcTask,

    runnable: LinkedList<SchedTaskAdapter>,

    deadline_awaiting: LinkedList<SchedTaskAdapter>,

    awaiting: LinkedList<SchedTaskAdapter>,

    stopped: LinkedList<SchedTaskAdapter>,
}

impl Default for Queues {
    fn default() -> Queues {
        let idle = Task::this();
        idle.set_state(TaskState::Idle);

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
    fn switch<F: FnOnce()>(&self, to: ArcTask, lock: SpinGuard<()>, f: Option<F>) {
        let _irq = lock.to_irq_guard();

        let prev = get_current();

        let prev_id = prev.tid();

        set_current(to);

        if let Some(f) = f {
            f();
        }

        // Careful with leaving Arcs on the stack as we may never be back on this frame
        // after switching... Should this happen, we would never drop strong count to 0
        let to = get_current();

        assert_eq!(prev_id, prev.tid());

        crate::kernel::sched::finalize();

        dbgln!(sched, "{} -> {}", prev.tid(), to.tid());

        if prev.tid() == to.tid() {
            return;
        }

        unsafe {
            if prev.state() == TaskState::Unused {
                // prev task is not gonna be scheduled again
                activate_task!(to);
            } else {
                switch!(prev, to);
            }
        }
    }

    fn schedule_check_deadline(&mut self) {
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
    fn debug_sched(&self, cpu: usize) {
        dbgln!(sched_v, "CPU: {}, c: {}| ", cpu, get_current().tid());

        dbgln!(sched_v, "runnable");
        for t in &self.runnable {
            let ptr = unsafe { t.arch_task().ctx.as_ptr() };
            dbgln!(
                sched_v,
                "{} {:x} {:p}",
                t.tid(),
                unsafe { t.arch_task().stack_top + t.arch_task().stack_size },
                ptr
            );
            unsafe {
                t.arch_task().ctx.as_ref().debug_stacktrace();
            }
        }
        dbgln!(sched_v, "awaiting");
        for t in &self.awaiting {
            let ptr = unsafe { t.arch_task().ctx.as_ptr() };
            dbgln!(
                sched_v,
                "{} {:x} {:p}",
                t.tid(),
                unsafe { t.arch_task().stack_top + t.arch_task().stack_size },
                ptr
            );
            unsafe {
                t.arch_task().ctx.as_ref().debug_stacktrace();
            }
        }
        dbgln!(sched_v, "deadline awaiting");
        for t in &self.deadline_awaiting {
            dbgln!(sched_v, "{} ", t.tid());
        }
    }

    fn schedule_next<F: FnOnce()>(&mut self, lock: SpinGuard<()>, f: Option<F>) -> bool {
        self.schedule_check_deadline();

        let current = get_current();

        let prev_id = current.tid();

        let to_run = self.runnable.pop_front().unwrap_or_else(|| {
            if current.tid() != self.idle_task.tid() && current.state() == TaskState::Runnable {
                current.me()
            } else {
                self.idle_task.clone()
            }
        });

        if current.tid() != to_run.tid()
            && !current.sched.is_linked()
            && current.state() == TaskState::Runnable
        {
            self.push_runnable(current.me(), false);
        }

        let to_run_tid = to_run.tid();

        self.switch(to_run, lock, f);

        prev_id != to_run_tid
    }

    fn reschedule(&mut self, lock: SpinGuard<()>) -> bool {
        self.schedule_next(lock, Option::<fn()>::None)
    }

    fn reschedule_exec<F: FnOnce()>(&mut self, lock: SpinGuard<()>, fun: F) -> bool {
        self.schedule_next(lock, Some(fun))
    }

    fn queue_task(&mut self, task: ArcTask, _lock: SpinGuard<()>) {
        //println!("queue task {}", task.id());
        self.push_runnable(task, false);
    }

    fn sleep(
        &mut self,
        time_ns: Option<usize>,
        flags: SleepFlags,
        lock: SpinGuard<()>,
    ) -> SignalResult<()> {
        let task = get_current();
        if task.locks() > 0 {
            dbgln!(
                warn,
                "sleeping while holding {} locks, tid: {}",
                task.locks(),
                task.tid()
            );
        }

        if task.has_pending_io() {
            task.set_has_pending_io(false);
            return Ok(());
        }

        if !flags.contains(SleepFlags::NON_INTERRUPTIBLE) && task.signals().has_pending() {
            return Err(SignalError::Interrupted);
        }

        if task.is_parent_terminating() {
            return Err(SignalError::Interrupted);
        }

        let pending = task.signals().pending();

        if pending > 0 {
            logln!("WARN sleep with pending signals: {:#x}", pending);
        }

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        // TODO: mark task as uninterruptible and dont wake it up on signals
        if let Some(time_ns) = time_ns {
            //dbgln!(task, "task {} pushed deadline awaiting", task.tid());
            self.push_deadline_awaiting(task.me(), time_ns);
        } else {
            //dbgln!(task, "task {} pushed awaiting", task.tid());
            self.push_awaiting(task.me());
        }

        self.reschedule(lock);

        if task.signals().pending() != pending {
            Err(SignalError::Interrupted)
        } else {
            Ok(())
        }
    }

    fn wake(&mut self, task: ArcTask, _lock: SpinGuard<()>) {
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
                dbgln!(task, "WARN: set pending io on UNUSED task {}", task.tid());
            }
            task.set_has_pending_io(true);
        }
    }

    fn wake_as_next(&mut self, task: ArcTask, _lock: SpinGuard<()>) {
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
        let task = get_current();

        assert_ne!(
            task.tid(),
            self.idle_task.tid(),
            "Idle task should not sleep"
        );

        dbgln!(task_stop, "Stopped task {}", task.tid());

        self.push_stopped(sig, task.me());

        self.reschedule_exec(lock, || {
            task.notify_stopped(sig);
        });
    }

    fn cont(&mut self, task: ArcTask, _lock: SpinGuard<()>) {
        if task.state() == TaskState::Stopped {
            assert!(task.sched.is_linked());
            let mut cursor = unsafe { self.stopped.cursor_mut_from_ptr(task.as_ref()) };

            if let Some(task) = cursor.remove() {
                dbgln!(task_stop, "Continued task {}", task.tid());
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
            "exit tid: {}, sc: {}, wc: {}, st: {:?} {}",
            current.tid(),
            ArcTask::strong_count(&current.me()),
            ArcTask::weak_count(&current.me()),
            status,
            current.exe().unwrap().full_path()
        );

        current.set_state(TaskState::Unused);

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

        task.set_state(TaskState::Unused);

        assert_eq!(task.state(), TaskState::Unused);
        assert_eq!(task.sched.is_linked(), false);
        assert!(!task.is_process_leader());

        dbgln!(
            task,
            "exit_thread tid: {}, sc: {}, wc: {}",
            task.tid(),
            ArcTask::strong_count(&task.me()),
            ArcTask::weak_count(&task.me()),
        );

        self.reschedule_exec(_lock, || {
            task.make_zombie(syscall_defs::waitpid::Status::Exited(0));
        });

        //logln!("UNEXPECTED EXIT THREAD TID {}", task.tid());

        unreachable!()
    }

    fn push_awaiting(&mut self, task: ArcTask) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(0);

        self.awaiting.push_back(task);
    }

    fn push_stopped(&mut self, _sig: usize, task: ArcTask) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Stopped);
        //task.set_sleep_until(0);

        self.stopped.push_back(task);
    }

    fn push_deadline_awaiting(&mut self, task: ArcTask, time_ns: usize) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        use crate::kernel::timer::current_ns;

        task.set_state(TaskState::AwaitingIo);
        task.set_sleep_until(current_ns() as usize + time_ns);

        self.deadline_awaiting.push_back(task);
    }

    fn push_runnable(&mut self, task: ArcTask, _continued: bool) {
        assert_eq!(task.sched.is_linked(), false);
        assert_ne!(task.tid(), self.idle_task.tid());

        task.set_state(TaskState::Runnable);
        //task.set_sleep_until(0);
        self.runnable.push_back(task);
    }

    fn push_runnable_front(&mut self, task: ArcTask, _continued: bool) {
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

        set_current(queue.idle_task.clone());

        crate::kernel::sched::register_task(&queue.idle_task);
    }

    fn reschedule(&self) -> bool {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.reschedule(lock)
    }

    fn current_task<'a>(&self) -> &'a Task {
        get_current()
    }

    fn queue_task(&self, task: ArcTask, alloc_cpu: bool) {
        if alloc_cpu {
            RRScheduler::alloc_cpu(&task);
        }

        if Self::maybe_do_ipi(&task, crate::kernel::ipi::queue) {
            return;
        }

        //assert!(task.is_on_this_cpu());

        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.queue_task(task, lock);
    }

    fn sleep(&self, until: Option<usize>, flags: SleepFlags) -> SignalResult<()> {
        let (lock, queue) = self.queues.this_cpu_mut();

        let lock = lock.lock_irq();

        queue.sleep(until, flags, lock)
    }

    fn wake(&self, task: ArcTask) {
        if Self::maybe_do_ipi(&task, crate::kernel::ipi::wake_up) {
            return;
        }

        //assert!(task.is_on_this_cpu());

        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.wake(task, lock);
    }

    fn wake_as_next(&self, task: ArcTask) {
        if Self::maybe_do_ipi(&task, crate::kernel::ipi::wake_up_next) {
            return;
        }

        //assert!(task.is_on_this_cpu());

        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.wake_as_next(task, lock);
    }

    fn cont(&self, task: ArcTask) {
        if Self::maybe_do_ipi(&task, crate::kernel::ipi::cont) {
            return;
        }

        //assert!(task.is_on_this_cpu());
        if task.is_process_leader() {
            task.cont_threads();
        }

        let (lock, queue) = self.queues.cpu_mut(task.on_cpu() as isize);

        let lock = lock.lock_irq();

        queue.cont(task.clone(), lock);
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

    fn debug(&self) {
        let cpu_count = crate::kernel::smp::cpu_count();

        for i in 0..cpu_count {
            let (l, q) = self.queues.cpu_mut(i as isize);

            let _lock = l.lock_irq();
            q.debug_sched(i);
        }
    }
}

impl RRScheduler {
    pub fn new() -> Arc<RRScheduler> {
        Arc::new(RRScheduler {
            queues: PerCpu::new_fn(|| (Spin::new(()), Queues::default())),
        })
    }

    fn alloc_cpu(task: &ArcTask) {
        task.set_on_cpu(task.tid() % crate::kernel::smp::cpu_count());
    }

    fn maybe_do_ipi(task: &ArcTask, fun: fn(&ArcTask)) -> bool {
        if task.is_on_this_cpu() {
            return false;
        }

        fun(task);
        true
    }
}
