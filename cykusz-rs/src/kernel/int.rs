pub use crate::arch::int::is_enabled;

pub fn disable() {
    crate::arch::int::disable();
}

pub fn enable() {
    crate::arch::int::enable();
}

pub fn enable_and_halt() {
    crate::arch::int::enable_and_halt();
}

pub fn finish() {
    crate::arch::int::end_of_int();
}

pub fn with_int_enabled<F: Fn() -> ()>(f: F) {
    let was_disabled = !is_enabled();

    enable();
    f();
    if was_disabled {
        disable();
    }
}
