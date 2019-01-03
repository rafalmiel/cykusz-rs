use arch::dev::lapic;
use arch::int;
use arch::raw::idt as ridt;
use kernel::sync::IrqLock;

struct Timer {
    pub handler: Option<fn () -> ()>,
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
    ::arch::dev::pit::early_sleep(ms);
}

pub extern "x86-interrupt" fn timer_handler(_frame: &mut ridt::ExceptionStackFrame) {
    let timer = &TIMER;
    if let Some(ref f) = timer.irq().handler {
        (f)();
    }
    int::end_of_int();
}
