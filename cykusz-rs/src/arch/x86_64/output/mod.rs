use core::fmt::Error;

use crate::kernel::sync::{Spin, SpinGuard};

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScreenChar {
    char: u8,
    color: ColorCode,
}

impl ScreenChar {
    pub fn new(char: u8, color: ColorCode) -> ScreenChar {
        ScreenChar { char, color }
    }
}

pub trait VideoDriver: Sync + Send {
    fn write_str(&self, _s: &str) -> Result<(), Error> {
        Ok(())
    }
    fn update_cursor(&self, _x: usize, _y: usize) {}
    fn clear(&self) {}

    fn dimensions(&self) -> (usize, usize) {
        (0, 0)
    }
    fn copy_txt_buffer(&self, _x: usize, _y: usize, _buf: &[ScreenChar]) {}
}

pub struct DefaultOutputWriter {}

static OUTPUT_WRITER: Spin<&'static dyn ConsoleWriter> = Spin::new(&DefaultOutputWriter {});

pub trait ConsoleWriter: Send + Sync {
    fn write_str(&self, s: &str) -> core::fmt::Result;
}

impl ConsoleWriter for DefaultOutputWriter {
    fn write_str(&self, s: &str) -> core::fmt::Result {
        video().write_str(s)
    }
}

pub fn video() -> SpinGuard<'static, &'static dyn VideoDriver> {
    VIDEO.lock()
}

type VideoDriverType = &'static dyn VideoDriver;

struct NoopVideoDriver {}

impl VideoDriver for NoopVideoDriver {}

static VIDEO: Spin<VideoDriverType> = Spin::new(&NoopVideoDriver {});

pub fn register_video_driver(driver: VideoDriverType) {
    *VIDEO.lock() = driver;
}

pub fn register_output_driver(driver: &'static dyn ConsoleWriter) {
    *OUTPUT_WRITER.lock() = driver;
}

pub fn init() {
    crate::arch::dev::serial::init();
    crate::drivers::video::vga::init();
    let w = video();
    w.clear()
}

struct Writer {}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let console = OUTPUT_WRITER.lock_irq();
        console.write_str(s)
    }
}

struct Log {}

impl core::fmt::Write for Log {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::arch::dev::serial::write(s);
        Ok(())
    }
}

pub fn write_fmt(args: core::fmt::Arguments) -> core::fmt::Result {
    core::fmt::write(&mut Writer {}, args)
}

pub fn log_fmt(args: core::fmt::Arguments) -> core::fmt::Result {
    core::fmt::write(&mut Log {}, args)
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

#[macro_export]
macro_rules! logln {
    ($fmt:expr) => (log!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (log!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        $crate::arch::output::log_fmt(format_args!($($arg)*)).unwrap();
    });
}
