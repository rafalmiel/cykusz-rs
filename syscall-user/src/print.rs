struct Writer {}
struct Debugger {}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        crate::print(s);
        Ok(())
    }
}

pub fn write_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    let mut w = Writer {};
    let r = ::core::fmt::write(&mut w, args);
    r
}

impl core::fmt::Write for Debugger {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        if let Err(_e) = crate::debug(s) {
            Err(core::fmt::Error)
        } else {
            Ok(())
        }
    }
}

pub fn debug_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    let mut w = Debugger {};
    let r = ::core::fmt::write(&mut w, args);
    r
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::print::write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! logln {
    ($fmt:expr) => (log!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (log!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        $crate::print::debug_fmt(format_args!($($arg)*)).unwrap();
    });
}
