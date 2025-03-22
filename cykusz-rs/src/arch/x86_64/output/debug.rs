use crate::kernel::sync::Spin;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
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

pub static DBG_LOCK: Spin<()> = Spin::new(());

#[thread_local]
pub static mut LOCK_CNT: usize = 0;

pub static LOG_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn toggle_log() {
    let enabled = LOG_ENABLED.load(Ordering::Relaxed);

    LOG_ENABLED.store(!enabled, Ordering::Relaxed);
}

#[cfg(feature = "logs")]
#[macro_export]
macro_rules! dbg {
    ($($log:ident)|*, $($arg:tt)*) => ({
        if !crate::kernel::smp::is_smp_initialised() ||
           !crate::arch::output::debug::LOG_ENABLED.load(core::sync::atomic::Ordering::Relaxed) {

        }
        else if let Some(loggers) = crate::arch::output::debug::loggers() {
            if stringify!($($log)|*).split('|').any(|e| { loggers.contains(e.trim()) }) {
                #[allow(unused_imports)]
                use crate::kernel::sync::LockApi;
                #[allow(unused_unsafe)]
                unsafe {
                    let _lock = if $crate::arch::output::debug::LOCK_CNT == 0 {
                        $crate::arch::output::debug::LOCK_CNT += 1;
                        Some(crate::arch::output::debug::DBG_LOCK.lock_irq())
                    } else {
                        $crate::arch::output::debug::LOCK_CNT += 1;
                        None
                    };
                    crate::arch::output::log_fmt(format_args!("[{}] ", crate::cpu_id())).unwrap();
                    crate::arch::output::log_fmt(format_args!($($arg)*)).unwrap();

                    $crate::arch::output::debug::LOCK_CNT -= 1;

                }
            }
        }
    });
}

#[macro_export]
macro_rules! dbgln {
    ($($log:ident)|+, $fmt:expr) => (dbg!($($log)|+, concat!(stringify!($($log)|+), ": ", $fmt, "\n")));
    ($($log:ident)|+, $fmt:expr, $($arg:tt)*) => (dbg!($($log)|+, concat!(stringify!($($log)|+), ": ", $fmt, "\n"), $($arg)*));
}
