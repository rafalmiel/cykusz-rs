
pub fn disable_ints() {
    ::arch::int::cli();
}

pub fn enable_ints() {
    ::arch::int::sti();
}
