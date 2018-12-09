use kernel::sync::IrqLock;
use kernel::task;
use kernel::mm::MappedAddr;

#[macro_export]
macro_rules! switch {
    ($ctx1:expr, $ctx2:expr) => (
        $crate::arch::task::switch(&mut $ctx1.arch_task, &$ctx2.arch_task);
    )
}

const TASK_COUNT: usize = 32;

struct Scheduler {
    sched_task: task::Task,
    tasks: [task::Task; TASK_COUNT],
    current: usize,
    previous: usize,
    pub initialised: bool
}

impl Scheduler {
    const fn empty() -> Scheduler {
        Scheduler {
            sched_task: task::Task::empty(),
            tasks: [task::Task::empty(); TASK_COUNT],
            current: 0,
            previous: 0,
            initialised: false,
        }
    }

    fn init(&mut self) {

        // Validate against any corruptions in TLS
        for t in self.tasks.iter() {
            t.assure_empty();
        }

        self.sched_task = task::Task::new_sched(scheduler_main);

        self.tasks[0].set_state(task::TaskState::Running);
        self.current = 0;
        self.initialised = true;

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
        if self.initialised {

            self.tasks[self.current].locks += 1;
        }
    }

    fn leave_critical_section(&mut self) {
        if self.initialised {
            let t = &mut self.tasks[self.current];

            t.locks -= 1;

            if t.state() == task::TaskState::ToReschedule && t.locks == 0 {
                t.set_state(task::TaskState::Running);
                reschedule();
            }
        }
    }
}

#[thread_local]
static SCHEDULER: IrqLock<Scheduler> = IrqLock::new(Scheduler::empty());

fn scheduler_main() {
    loop {
        let scheduler = &SCHEDULER;
        scheduler.irq().schedule_next();
    }
}

pub fn reschedule() -> bool {
    let scheduler = &SCHEDULER;
    let a = scheduler.irq().reschedule();
    a
}

pub fn task_finished() {
    print!("f,");
    let scheduler = &SCHEDULER;
    scheduler.irq().current_task_finished();
}

pub fn create_task(fun: fn()) {
    let scheduler = &SCHEDULER;
    scheduler.irq().add_task(fun);
}

pub fn create_user_task(fun: MappedAddr, code_size: u64, stack: usize) {
    let scheduler = &SCHEDULER;
    scheduler.irq().add_user_task(fun, code_size as usize, stack);
}

pub fn enter_critical_section() -> bool {
    if ::kernel::tls::is_ready() {
        let scheduler = &SCHEDULER;
        scheduler.irq().enter_critical_section();
        return true;
    }

    return false;
}

pub fn leave_critical_section() {
    if ::kernel::tls::is_ready() {
        let scheduler = &SCHEDULER;
        scheduler.irq().leave_critical_section();
    }
}

pub fn init() {
    let scheduler = &SCHEDULER;
    scheduler.irq().init();
}

