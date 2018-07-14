pub fn init() {
    ::arch::smp::init();
}

pub fn cpu_count() -> usize {
    ::arch::smp::cpu_count()
}
