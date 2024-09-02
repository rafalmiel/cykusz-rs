pub fn init() {}

#[macro_export]
macro_rules! dbg {
    ($($log:ident)|*, $($arg:tt)*) => ({
        crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! dbgln {
    ($($log:ident)|*, $fmt:expr) => (dbg!($($log)|*, concat!(stringify!($($log)|*), ": ", $fmt, "\n")));
    ($($log:ident)|*, $fmt:expr, $($arg:tt)*) => (dbg!($($log)|*, concat!(stringify!($($log)|*), ": ", $fmt, "\n"), $($arg)*));
}
