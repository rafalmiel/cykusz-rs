pub fn setup() {
    ::arch::timer::setup(timer_handler);
}

pub fn start() {
    ::arch::timer::start();
}

pub fn reset_counter() {
    ::arch::timer::reset_counter();
}

fn timer_handler() {
    ::kernel::sched::reschedule();
}

pub fn early_sleep(ms: u64) {
    ::arch::timer::early_sleep(ms);
}
