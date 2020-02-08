pub fn setup() {
    crate::arch::timer::setup(timer_handler);
}

pub fn start() {
    crate::arch::timer::start();
}

pub fn reset_counter() {
    crate::arch::timer::reset_counter();
}

fn timer_handler() {
    //println!("Timer handler");
    crate::kernel::sched::reschedule();
}

pub fn early_sleep(ms: u64) {
    crate::arch::timer::early_sleep(ms);
}