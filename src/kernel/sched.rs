use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;

use core::sync::atomic::{AtomicUsize, AtomicBool};
use core::sync::atomic::Ordering;

use spin::Once;

use kernel::mm::MappedAddr;
use kernel::sync::{Mutex, MutexGuard};
use kernel::task::{Task, TaskState};
use alloc::vec::Vec;
use kernel::utils::PerCpu;
use core::cell::UnsafeCell;
use core::borrow::BorrowMut;

#[macro_export]
macro_rules! switch {
    ($ctx1: expr, $ctx2: expr) => (
        $crate::arch::task::switch(&mut $ctx1.arch_task, &$ctx2.arch_task);
    )
}

static NEW_TASK_ID: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static LOCK_PROTECTION: AtomicBool = AtomicBool::new(false);

#[thread_local]
static LOCK_PROTECTION_ENTERED: AtomicBool = AtomicBool::new(false);

#[thread_local]
static CURRENT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

pub fn new_task_id() -> usize {
    NEW_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

struct CpuQueue {
    sched_task: Arc<Task>,
    tasks: Vec<Arc<Task>>,
    current: usize,
    previous: usize,
}

struct CpuQueues {
    cpu_queues_locks: PerCpu<Mutex<()>>,
    cpu_queues: PerCpu<UnsafeCell<CpuQueue>>,
}

struct TaskContainer {
    tasks: Mutex<BTreeMap<usize, Arc<Task>>>
}

struct Scheduler {
    tasks: TaskContainer,
    cpu_queues: CpuQueues,
}

unsafe impl Sync for CpuQueues{}

impl Default for CpuQueue {
    fn default() -> CpuQueue {
        let mut this = CpuQueue {
            sched_task: Arc::new(Task::new_sched(scheduler_main)),
            tasks: Vec::new(),
            current: 0,
            previous: 0,
        };

        this.tasks.push(Arc::new(Task::this()));

        this
    }
}

impl Default for CpuQueues {
    fn default() -> CpuQueues {
        CpuQueues {
            cpu_queues_locks: PerCpu::new_fn(|| {
                Mutex::<()>::new(())
            }),
            cpu_queues: PerCpu::new_fn(|| {
                UnsafeCell::new(CpuQueue::default())
            })
        }
    }
}

impl Default for TaskContainer {
    fn default() -> TaskContainer {
        TaskContainer {
            tasks: Mutex::new(BTreeMap::new())
        }
    }
}

impl Default for Scheduler {
    fn default() -> Scheduler {
        Scheduler {
            tasks: TaskContainer::default(),
            cpu_queues: CpuQueues::default(),
        }
    }
}

impl CpuQueue {

    fn switch(&self, to: Arc<Task>, lock: MutexGuard<()>) {
        drop(lock);
        unsafe {
            switch!(CpuQueue::as_mut(&self.sched_task), &to);
        }
    }

    fn switch_to_sched(&self, from: Arc<Task>, lock: MutexGuard<()>) {
        drop(lock);
        unsafe {
            switch!(CpuQueue::as_mut(&from), &self.sched_task);
        }
    }

    unsafe fn schedule_next(&mut self, sched_lock: MutexGuard<()>) {

        if self.tasks[self.current].state == TaskState::ToDelete {
            self.remove_task(self.current);
            if self.current != 0 {
                self.current -= 1;
            }
            self.schedule_next(sched_lock);
            return;
        } else if self.tasks[self.current].locks > 0 {
            self.task_mut_at(self.current).state = TaskState::ToReschedule;
            self.switch(self.tasks[self.current].clone(), sched_lock);
            return;
        }

        if self.tasks.len() == 1 {
            self.switch(self.tasks[0].clone(), sched_lock);
            return;
        }

        let len = self.tasks.len();

        let mut c = (self.current % (len - 1)) + 1;
        let mut loops = 0;

        let found = loop {
            if self.tasks[c].state == TaskState::Runnable {
                break Some(c);
            } else if c == self.current && self.tasks[self.current].state == TaskState::Running {
                break Some(self.current);
            } else if loops == len - 1 {
                break Some(0);
            }

            c = (c % (len - 1)) + 1;
            loops += 1;
        }.expect("SCHEDULER BUG");

        if self.tasks[self.current].state == TaskState::Running {
            self.task_mut_at(self.current).state = TaskState::Runnable;
        }

        self.task_mut_at(found).state = TaskState::Running;

        self.previous = self.current;
        self.current = found;
        CURRENT_TASK_ID.store(found, Ordering::SeqCst);

        ::kernel::int::finish();

        ::kernel::timer::reset_counter();

        self.switch( self.tasks[found].clone(), sched_lock);

    }

    unsafe fn as_mut(task: &Task) -> &mut Task {
        &mut *(task as *const Task as *mut Task)
    }

    unsafe fn task_mut_at(&mut self, idx: usize) -> &mut Task {
        CpuQueue::as_mut(&self.tasks[idx])
    }

    fn reschedule(&mut self, sched_lock: MutexGuard<()>) -> bool {
        let current = self.tasks[self.current].clone();

        self.switch_to_sched(current, sched_lock);

        return self.current != self.previous;
    }

    fn enter_critical_section(&mut self) {
        let c = self.current;

        unsafe {
            self.task_mut_at(c)
        }.locks += 1;
    }

    fn leave_critical_section(&mut self, mutex: MutexGuard<()>) {
        let c = self.current;

        let t = unsafe {
            self.task_mut_at(c)
        };

        t.locks -= 1;

        if t.locks == 0 && t.state == TaskState::ToReschedule {
            t.state = TaskState::Running;

            drop(mutex);
            reschedule();
        }
    }

    fn current_task_finished(&mut self, lock: MutexGuard<()>) {
        unsafe {
            LOCK_PROTECTION_ENTERED.store(true, Ordering::SeqCst);
            self.task_mut_at(self.current).deallocate();
            LOCK_PROTECTION_ENTERED.store(false, Ordering::SeqCst);
        }

        if !self.reschedule(lock) {
            panic!("Task finished but still running?");
        }
    }

    fn add_task(&mut self, task: Arc<Task>) {
        LOCK_PROTECTION_ENTERED.store(true, Ordering::SeqCst);
        self.tasks.push(task);
        LOCK_PROTECTION_ENTERED.store(false, Ordering::SeqCst);
    }

    fn remove_task(&mut self, idx: usize) {
        LOCK_PROTECTION_ENTERED.store(true, Ordering::SeqCst);
        self.tasks.remove(idx);
        LOCK_PROTECTION_ENTERED.store(false, Ordering::SeqCst);
    }
}

impl CpuQueues {

    unsafe fn this_cpu_queue(&self) -> &mut CpuQueue {
        (&mut *(self.cpu_queues.this_cpu_mut().get()))
    }

    fn schedule_next(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().schedule_next(mutex);
        }
    }

    fn reschedule(&self) -> bool {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().reschedule(mutex)
        }
    }

    fn enter_critical_section(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().enter_critical_section();
        }
    }

    fn leave_critical_section(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().leave_critical_section(mutex);
        }

    }

    fn add_task(&self, task: Arc<Task>) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();
        unsafe {
            self.this_cpu_queue().add_task(task);
        }
    }

    fn current_task_finished(&self) {
        let mutex = self.cpu_queues_locks.this_cpu().lock_irq();

        unsafe {
            self.this_cpu_queue().current_task_finished(mutex);
        }
    }
}

impl TaskContainer {
    fn add_task(&self, fun: fn()) -> Arc<Task> {
        let task = Arc::new(Task::new_kern(fun));

        self.tasks.lock().insert(task.id, task.clone());

        task
    }

    fn add_user_task(&self, fun: MappedAddr, code_size: usize, stack: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_user(fun, code_size, stack));

        self.tasks.lock().insert(task.id, task.clone());

        task
    }

    fn remove_task(&self, id: usize) {
        self.tasks.lock().remove(&id);
    }
}

impl Scheduler {

    fn add_task(&self, fun: fn()) {
        let task = self.tasks.add_task(fun);

        self.cpu_queues.add_task(task);
    }

    fn add_user_task(&self, fun: MappedAddr, code_size: usize, stack: usize) {
        let task = self.tasks.add_user_task(fun, code_size, stack);

        self.cpu_queues.add_task(task);
    }

    fn schedule_next(&self) {
        self.cpu_queues.schedule_next();
    }

    fn reschedule(&self) -> bool {
        self.cpu_queues.reschedule()
    }

    fn enter_critical_section(&self) {
        self.cpu_queues.enter_critical_section();
    }

    fn leave_critical_section(&self) {
        self.cpu_queues.leave_critical_section();
    }

    fn current_task_finished(&self) {
        self.tasks.remove_task(CURRENT_TASK_ID.load(Ordering::SeqCst));
        self.cpu_queues.current_task_finished();
    }
}

static SCHEDULER: Once<Scheduler> = Once::new();

fn scheduler() -> &'static Scheduler {
    SCHEDULER.try().expect("Scheduler not initialized")
}

fn scheduler_main() {
    loop {
        unsafe {
            scheduler().schedule_next();
        }
    }
}

pub fn reschedule() -> bool {
    scheduler().reschedule()
}

pub fn task_finished() {
    scheduler().current_task_finished();
}

pub fn create_task(fun: fn()) {
    scheduler().add_task(fun);
}

pub fn create_user_task(fun: MappedAddr, code_size: u64, stack: usize) {
    scheduler().add_user_task(fun, code_size as usize, stack);
}

fn lock_protection_ready() -> bool {
    ::kernel::tls::is_ready()
        && LOCK_PROTECTION.load(Ordering::SeqCst)
        && !LOCK_PROTECTION_ENTERED.load(Ordering::SeqCst)
}

pub fn enter_critical_section() -> bool {
    if lock_protection_ready() {

        scheduler().enter_critical_section();

        return true;
    }

    return false;
}

pub fn leave_critical_section() {
    if lock_protection_ready() {

        scheduler().leave_critical_section();
    }
}

pub fn init() {
    SCHEDULER.call_once(|| {
        Scheduler::default()
    });
}

pub fn enable_lock_protection() {
    LOCK_PROTECTION.store(true, Ordering::SeqCst);
}

