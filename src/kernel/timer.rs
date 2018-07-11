pub fn setup() {
    ::arch::timer::setup_timer(timer_handler);
}

pub fn start() {
    ::arch::timer::start_timer();
}

fn timer_handler() {
    unsafe {
        print!("{}", ::CPU_ID);
    }
}
