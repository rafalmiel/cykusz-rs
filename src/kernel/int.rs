pub use arch::int::is_int_enabled;

pub fn disable() {
    ::arch::int::cli();
}

pub fn enable() {
    ::arch::int::sti();
}
