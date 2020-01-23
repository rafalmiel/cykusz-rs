struct Writer {}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        crate::syscall::print(s);
        Ok(())
    }
}

pub fn write_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    let mut w = Writer {};
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
        crate::print::write_fmt(format_args!($($arg)*)).unwrap();
    });
}
