use kernel::sync::IrqLock;
use kernel::task;
use kernel::mm::heap::allocate as heap_allocate;
use kernel::mm::MappedAddr;

#[macro_export]
macro_rules! switch {
    ($ctx1:expr, $ctx2:expr) => (
        $crate::arch::task::switch(&mut $ctx1.arch_task, &$ctx2.arch_task);
    )
}

const TASK_COUNT: usize = 32;

struct CpuTasks {
    sched_task: task::Task,
    tasks: [task::Task; TASK_COUNT],
    current: usize,
    previous: usize,
}

struct CpuTasksPtr(*mut CpuTasks);

unsafe impl Send for CpuTasksPtr{}

impl CpuTasks {
    fn init(&mut self) {
        self.sched_task = task::Task::empty();

        for i in 0..TASK_COUNT {
            self.tasks[i] = task::Task::empty();
        }

        self.tasks[0].set_state(task::TaskState::Running);

        self.sched_task = task::Task::new_sched(scheduler_main);

        self.current = 0;
        self.previous = 0;
    }

    fn schedule_next(&mut self) {
        //::bochs();
        if self.tasks[self.current].state() == task::TaskState::ToDelete {

            self.tasks[self.current].deallocate();
            return;
        } else if self.tasks[self.current].locks > 0 {

            self.tasks[self.current].set_state(task::TaskState::ToReschedule);
            switch!(self.sched_task, self.tasks[self.current]);
            return;
        }

        let mut c = (self.current % (TASK_COUNT - 1)) + 1;
        let mut loops = 0;

        let found = loop {
            if self.tasks[c].state() == task::TaskState::Runnable {

                break Some(c);
            } else if c == self.current && self.tasks[self.current].state() == task::TaskState::Running {

                break Some(self.current);
            } else if loops == TASK_COUNT - 1 {

                break Some(0)
            }

            c = (c % (TASK_COUNT - 1)) + 1;
            loops += 1;
        }.expect("SCHEDULER BUG");

        if self.tasks[self.current].state() == task::TaskState::Running {
            self.tasks[self.current].set_state(task::TaskState::Runnable);
        }

        self.tasks[found].set_state(task::TaskState::Running);
        self.previous = self.current;
        self.current = found;

        ::kernel::int::finish();

        ::kernel::timer::reset_counter();
        switch!(self.sched_task, self.tasks[found]);
    }

    fn reschedule(&mut self) -> bool {
        switch!(self.tasks[self.current], self.sched_task);
        return self.previous != self.current
    }

    fn current_task_finished(&mut self) {
        self.tasks[self.current].set_state(task::TaskState::ToDelete);
        if !self.reschedule() {
            panic!("task_finished but still running?");
        }
    }

    fn add_task(&mut self, fun: fn()) {
        for i in 1..32 {
            if self.tasks[i].state() == task::TaskState::Unused {
                self.tasks[i] = task::Task::new_kern(fun);
                return;
            }
        }

        panic!("Sched: Too many tasks!");
    }

    fn add_user_task(&mut self, fun: MappedAddr, code_size: usize, stack: usize) {
        for i in 1..32 {
            if self.tasks[i].state() == task::TaskState::Unused {
                self.tasks[i] = task::Task::new_user(fun, code_size, stack);
                return;
            }
        }

        panic!("Sched: Too many tasks!");
    }

    fn enter_critical_section(&mut self) {
        self.tasks[self.current].locks += 1;
    }

    fn leave_critical_section(&mut self) {
        let t = &mut self.tasks[self.current];

        t.locks -= 1;

        if t.state() == task::TaskState::ToReschedule && t.locks == 0 {
            t.set_state(task::TaskState::Running);
            reschedule();
        }
    }

    fn current(&mut self) -> &'static mut task::Task {
        unsafe {
            &mut *(&mut self.tasks[self.current] as *mut _)
        }
    }
}

struct Tasks {
    tasks: CpuTasksPtr,
}

impl Tasks {
    const fn empty() -> Tasks {
        Tasks {
            tasks: CpuTasksPtr(::core::ptr::null_mut()),
        }
    }

    fn init(&mut self) {
        use ::kernel::smp::cpu_count;
        use ::core::mem::size_of;

        self.tasks = CpuTasksPtr(heap_allocate(size_of::<CpuTasks>() * cpu_count()).expect("Out of mem") as *mut CpuTasks);

        for i in 0..cpu_count() {
            self.at_cpu(i as isize).init();
        }
    }

    fn at_cpu(&mut self, cpu: isize) -> &mut CpuTasks {
        unsafe {
            &mut *self.tasks.0.offset(cpu)
        }
    }

    fn at_this_cpu(&mut self) -> &'static mut CpuTasks {
        unsafe {
            &mut *self.tasks.0.offset(::CPU_ID as isize)
        }
    }
}

struct Scheduler {
    tasks: Tasks,
    pub initialised: bool
}

impl Scheduler {
    const fn empty() -> Scheduler {
        Scheduler {
            tasks: Tasks::empty(),
            initialised: false,
        }
    }

    fn init(&mut self) {
        self.tasks.init();

        self.initialised = true;
    }

    fn this_cpu_tasks(&mut self) -> &mut CpuTasks {
        self.tasks.at_this_cpu()
    }

    fn schedule_next(&mut self) {
        self.this_cpu_tasks().schedule_next();
    }

    fn reschedule(&mut self) -> bool {
        self.this_cpu_tasks().reschedule()
    }

    fn current_task_finished(&mut self) {
        self.this_cpu_tasks().current_task_finished()
    }

    fn add_task(&mut self, fun: fn()) {
        self.this_cpu_tasks().add_task(fun);
    }

    fn add_user_task(&mut self, fun: MappedAddr, code_size: usize, stack: usize) {
        self.this_cpu_tasks().add_user_task(fun, code_size, stack)
    }

    fn enter_critical_section(&mut self) {
        if self.initialised {
            self.this_cpu_tasks().enter_critical_section()
        }
    }

    fn leave_critical_section(&mut self) {
        if self.initialised {
            self.this_cpu_tasks().leave_critical_section()
        }
    }

    fn current(&mut self) -> &'static mut task::Task {
        self.this_cpu_tasks().current()
    }

}

static SCHEDULER: IrqLock<Scheduler> = IrqLock::new(Scheduler::empty());

fn scheduler_main() {
    loop {
        SCHEDULER.irq().schedule_next();
    }
}

pub fn reschedule() -> bool {
    let a = SCHEDULER.irq().reschedule();
    a
}

pub fn task_finished() {
    print!("f,");
    SCHEDULER.irq().current_task_finished();
}

pub fn create_task(fun: fn()) {
    SCHEDULER.irq().add_task(fun);
}

pub fn create_user_task(fun: MappedAddr, code_size: u64, stack: usize) {
    SCHEDULER.irq().add_user_task(fun, code_size as usize, stack);
}

pub fn enter_critical_section() -> bool {
    if ::kernel::tls::is_ready() {
        SCHEDULER.irq().enter_critical_section();
        return true;
    }

    return false;
}

pub fn leave_critical_section() {
    if ::kernel::tls::is_ready() {
        SCHEDULER.irq().leave_critical_section();
    }
}

pub fn current() -> &'static mut task::Task {
    SCHEDULER.irq().current()
}

pub fn init() {
    SCHEDULER.irq().init();
}

