pub use arch::int::is_enabled;

pub fn disable() {
    ::arch::int::disable();
}

pub fn enable() {
    ::arch::int::enable();
}

pub fn enable_and_halt() {
    ::arch::int::enable_and_halt();
}

pub fn finish() {
    ::arch::int::end_of_int();
}
