use crate::arch::raw::idt as ridt;
use crate::arch::int;
use crate::arch::dev::lapic;
use crate::kernel::sync::Mutex;

struct Timer {
    pub handler: Option<fn () -> ()>,
}

static TIMER: Mutex<Timer> = Mutex::new(Timer { handler: None });

pub fn setup_timer(fun: fn()) {
    let mut tmr = TIMER.lock();

    tmr.handler = Some(fun);

    lapic::start_timer(timer_handler);
}

pub fn start_timer() {
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
