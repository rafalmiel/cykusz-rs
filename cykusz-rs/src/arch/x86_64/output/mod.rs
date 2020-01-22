use crate::kernel::sync::{Mutex, MutexGuard};
use core::fmt::Error;

pub trait ConsoleDriver : Sync + Send {
    fn write_str(&mut self, s: &str) -> Result<(), Error>;
    fn clear(&mut self);
    fn remove_last_n(&mut self, n: usize);
}

impl core::fmt::Write for dyn ConsoleDriver {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        <Self as ConsoleDriver>::write_str(self, s)
    }
}

type ConsoleDriverType = &'static mut dyn ConsoleDriver;

static WRITER: Mutex<Option<ConsoleDriverType>> = Mutex::new(None);

pub fn register_console_driver(driver: ConsoleDriverType) {
    *WRITER.lock() = Some(driver);
}

pub fn writer<'a>() -> MutexGuard<'a, Option<ConsoleDriverType>> {
    let l = WRITER.lock_irq();
    l
}

pub fn write_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    let mut w = writer();
    let w = w.as_mut().unwrap();
    ::core::fmt::write(w, args)
}

pub fn init() {
    crate::drivers::video::vga::init();
    let mut w = writer();
    w.as_mut().unwrap().clear();
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::arch::output::write_fmt(format_args!($($arg)*)).unwrap();
    });
}
