use alloc::string::String;
use hashbrown::HashSet;
use spin::Once;

static ENABLED_LOGGERS: Once<HashSet<String>> = Once::new();

pub fn init() {
    use alloc::vec::Vec;
    ENABLED_LOGGERS.call_once(|| {
        if let Some(loggers) = crate::kernel::params::get("logs") {
            let mut ret = HashSet::new();
            for e in loggers.split(",").collect::<Vec<&str>>().iter() {
                ret.insert(String::from(*e));
            }
            ret
        } else {
            HashSet::new()
        }
    });
}

pub fn loggers() -> Option<&'static HashSet<String>> {
    ENABLED_LOGGERS.get()
}

#[cfg(feature = "logs")]
#[macro_export]
macro_rules! dbg {
    ($log:ident, $($arg:tt)*) => ({
        if let Some(loggers) = crate::arch::output::debug::loggers() {
            if loggers.contains(stringify!($log)) {
                crate::arch::output::log_fmt(format_args!($($arg)*)).unwrap();
            }
        }
    });
}

#[macro_export]
macro_rules! dbgln {
    ($log:ident, $fmt:expr) => (dbg!($log, concat!(stringify!($log), ": ", $fmt, "\n")));
    ($log:ident, $fmt:expr, $($arg:tt)*) => (dbg!($log, concat!(stringify!($log), ": ", $fmt, "\n"), $($arg)*));
}
