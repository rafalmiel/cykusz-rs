pub fn setup() {
    ::arch::timer::setup(timer_handler);
}

pub fn start() {
    ::arch::timer::start();
}

fn timer_handler() {
    unsafe {
        print!("{}", ::CPU_ID);
    }
}

pub fn early_sleep(ms: u64) {
    ::arch::timer::early_sleep(ms);
}
