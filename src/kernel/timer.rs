pub fn setup() {
    ::arch::timer::setup(timer_handler);
}

fn timer_handler() {
    ::kernel::sched::reschedule();
}

pub fn early_sleep(ms: u64) {
    ::arch::timer::early_sleep(ms);
}
