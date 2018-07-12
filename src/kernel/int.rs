pub use arch::int::is_enabled;

pub fn disable() {
    ::arch::int::disable();
}

pub fn enable() {
    ::arch::int::enable();
}
