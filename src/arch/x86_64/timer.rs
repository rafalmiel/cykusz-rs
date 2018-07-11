use arch::raw::idt as ridt;
use arch::int;
use arch::dev::lapic;
use kernel::sync::Mutex;

struct Timer {
    pub handler: Option<fn () -> ()>,
}

static TIMER: Mutex<Timer> = Mutex::new(Timer { handler: None });

pub fn setup(fun: fn()) {
    let mut tmr = TIMER.lock();

    tmr.handler = Some(fun);

    lapic::start_timer(timer_handler);
}

pub fn start() {
    lapic::start_timer(timer_handler);
}

pub extern "x86-interrupt" fn timer_handler(_frame: &mut ridt::ExceptionStackFrame) {
    {
        let tmr = TIMER.lock_irq();

        if let Some(ref f) = tmr.handler {
            (f)();
        }
    }
    int::end_of_int();
}

pub fn early_sleep(ms: u64) {
    ::arch::dev::pit::early_sleep(ms);
}
