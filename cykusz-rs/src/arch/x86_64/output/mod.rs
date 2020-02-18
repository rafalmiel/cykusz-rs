use core::fmt::Error;

use crate::kernel::sync::{Mutex, MutexGuard};

pub trait ConsoleDriver: Sync + Send {
    fn write_str(&self, s: &str) -> Result<(), Error>;
    fn clear(&self);
    fn remove_last_n(&self, n: usize);
}

pub struct OutputWriter {}

static OUTPUT_WRITER: Mutex<OutputWriter> = Mutex::new(OutputWriter {});

impl core::fmt::Write for OutputWriter {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        <Self as ConsoleDriver>::write_str(self, s)
    }
}

impl ConsoleDriver for OutputWriter {
    fn write_str(&self, s: &str) -> Result<(), Error> {
        WRITER
            .lock_irq()
            .expect("Output driver not initialised")
            .write_str(s)
    }

    fn clear(&self) {
        WRITER
            .lock_irq()
            .expect("Output driver not initialised")
            .clear()
    }

    fn remove_last_n(&self, n: usize) {
        WRITER
            .lock_irq()
            .expect("Output driver not initialised")
            .remove_last_n(n)
    }
}

pub fn writer<'a>() -> MutexGuard<'a, OutputWriter> {
    OUTPUT_WRITER.lock_irq()
}

type ConsoleDriverType = &'static dyn ConsoleDriver;

static WRITER: Mutex<Option<ConsoleDriverType>> = Mutex::new(None);

pub fn register_console_driver(driver: ConsoleDriverType) {
    *WRITER.lock() = Some(driver);
}

pub fn init() {
    crate::arch::dev::serial::init();
    crate::drivers::video::vga::init();
    let w = writer();
    w.clear()
}

pub fn write_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    // Need to lock_irq if we want to print inside interrupts
    let mut output = OUTPUT_WRITER.lock_irq();
    ::core::fmt::write(&mut *output, args)
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
