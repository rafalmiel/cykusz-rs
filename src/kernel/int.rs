pub use crate::arch::int::is_int_enabled;

pub fn disable() {
    crate::arch::int::cli();
}

pub fn enable() {
    crate::arch::int::sti();
}
