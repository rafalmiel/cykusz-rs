use crate::arch::dev::lapic;
use crate::kernel::sync::IrqLock;

struct Timer {
    pub handler: Option<fn() -> ()>,
}

#[thread_local]
static TIMER: IrqLock<Timer> = IrqLock::new(Timer { handler: None });

pub fn setup(fun: fn()) {
    let timer = &TIMER;
    let mut tmr = timer.irq();
    tmr.handler = Some(fun);

    lapic::setup_timer(timer_handler);
}

pub fn start() {
    lapic::start_timer(true);
}

pub fn reset_counter() {
    lapic::reset_timer_counter();
}

pub fn early_sleep(ms: u64) {
    crate::arch::dev::pit::early_busy_sleep(ms);
}

pub fn busy_sleep(ns: u64) {
    crate::arch::dev::hpet::busy_sleep(ns)
}

pub fn current_ns() -> u64 {
    crate::arch::dev::hpet::current_ns()
}

fn timer_handler() -> bool {
    let timer = &TIMER;
    if let Some(ref f) = timer.irq().handler {
        (f)();
    }
    true
}
