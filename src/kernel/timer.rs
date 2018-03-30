pub fn setup_timer() {
    ::arch::timer::setup_timer(timer_handler);
}

pub fn start_timer() {
    ::arch::timer::start_timer();
}

fn timer_handler() {
    unsafe {
        print!("{}", ::CPU_ID);
    }
}
