pub fn setup() {
    crate::arch::timer::setup_timer(timer_handler);
}

pub fn start() {
    crate::arch::timer::start_timer();
}

fn timer_handler() {
    unsafe {
        print!("{}", crate::CPU_ID);
    }
}
