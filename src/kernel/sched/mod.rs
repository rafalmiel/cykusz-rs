use kernel::sync::IrqLock;
pub mod task;

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
    pub initialisd: bool
}

impl Scheduler {
    const fn empty() -> Scheduler {
        Scheduler {
            sched_task: task::Task::empty(),
            tasks: [task::Task::empty(); TASK_COUNT],
            current: 0,
            initialisd: false,
        }
    }

    fn init(&mut self) {
        self.sched_task = task::Task::new_sched(scheduler_main);

        //TODO: For some reason tasks array was not initialised to 0's, need to look into it
        self.tasks = [task::Task::empty(); 32];
        self.tasks[0].state = task::TaskState::Running;
        self.current = 0;
        self.initialisd = true;
    }

    fn schedule_next(&mut self) {
        if self.tasks[self.current].state == task::TaskState::ToDelete {

            self.tasks[self.current].deallocate();
            return;
        } else if self.tasks[self.current].locks > 0 {

            self.tasks[self.current].state = task::TaskState::ToReschedule;
            switch!(self.sched_task, self.tasks[self.current]);
            return;
        }

        let mut c = (self.current % (TASK_COUNT - 1)) + 1;

        if let Some(found) = loop {
            if self.tasks[c].state == task::TaskState::Runnable {

                break Some(c);
            } else if c == self.current {

                break Some(0);
            }

            c = (c % (TASK_COUNT - 1)) + 1;
        } {
            if self.tasks[self.current].state == task::TaskState::Running {
                self.tasks[self.current].state = task::TaskState::Runnable;
            }

            self.tasks[found].state = task::TaskState::Running;
            self.current = found;

            ::kernel::int::finish();
            switch!(self.sched_task, self.tasks[found]);
        } else {
            panic!("SCHED BUG");
        }
    }

    fn reschedule(&mut self) {
        switch!(self.tasks[self.current], self.sched_task);
    }

    fn current_task_finished(&mut self) {
        self.tasks[self.current].state = task::TaskState::ToDelete;
        self.reschedule();
    }

    fn add_task(&mut self, fun: fn()) {
        for i in 1..32 {
            if self.tasks[i].state == task::TaskState::Unused {
                self.tasks[i] = task::Task::new_kern(fun);
                return;
            }
        }

        panic!("Sched: Too many tasks!");
    }

    fn enter_critical_section(&mut self) {
        if self.initialisd {
            unsafe {
                asm!("xchg %bx, %bx");
            }

            self.tasks[self.current].locks += 1;
        }
    }

    fn leave_critical_section(&mut self) {
        if self.initialisd {
            unsafe {
                asm!("xchg %bx, %bx");
            }
            let t = &mut self.tasks[self.current];

            t.locks -= 1;

            if t.state == task::TaskState::ToReschedule && t.locks == 0 {
                t.state = task::TaskState::Running;
                reschedule();
            }
        }
    }
}

#[thread_local]
static SCHEDULER: IrqLock<Scheduler> = IrqLock::new(Scheduler::empty());

pub fn scheduler_main() {
    loop {
        let scheduler = &SCHEDULER;
        scheduler.irq().schedule_next();
    }
}

pub fn reschedule() {
    let scheduler = &SCHEDULER;
    scheduler.irq().reschedule();
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

