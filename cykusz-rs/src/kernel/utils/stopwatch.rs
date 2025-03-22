use crate::kernel::timer::current_ns;

pub struct StopWatch {
    start_time: u64,
    name: &'static str,
}

impl StopWatch {
    pub fn new(name: &'static str) -> StopWatch {
        StopWatch {
            start_time: current_ns(),
            name,
        }
    }
}

impl Drop for StopWatch {
    fn drop(&mut self) {
        let end_time = current_ns();
        let time = (end_time - self.start_time) / 1000;

        dbgln!(stopwatch, "{} took {} us", self.name, time);
    }
}
