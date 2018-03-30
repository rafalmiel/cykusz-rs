pub use arch::int::is_int_enabled;

pub fn disable_ints() {
    ::arch::int::cli();
}

pub fn enable_ints() {
    ::arch::int::sti();
}
