#[cfg(feature = "logs")]
#[macro_use]
pub mod debug;

#[cfg(not(feature = "logs"))]
#[macro_use]
pub mod debug_disabled;

#[cfg(not(feature = "logs"))]
pub use debug_disabled as debug;

use core::fmt::Error;

use bit_field::BitField;

use crate::drivers::multiboot2::framebuffer_info::FramebufferInfo;
use crate::kernel::sync::{LockApi, Spin, SpinGuard};

#[derive(Copy, Clone, Debug)]
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

impl Color {
    pub fn brighten(self) -> Color {
        match self {
            Color::Black => Color::DarkGray,
            Color::Blue => Color::LightBlue,
            Color::Green => Color::LightGreen,
            Color::Cyan => Color::LightCyan,
            Color::Red => Color::LightRed,
            Color::Magenta => Color::Pink,
            Color::Brown => Color::Yellow,
            Color::LightGray => Color::White,
            _ => self,
        }
    }

    pub fn dim(self) -> Color {
        match self {
            Color::DarkGray => Color::Black,
            Color::LightBlue => Color::Blue,
            Color::LightGreen => Color::Green,
            Color::LightCyan => Color::Cyan,
            Color::LightRed => Color::Red,
            Color::Pink => Color::Magenta,
            Color::Yellow => Color::Brown,
            Color::White => Color::LightGray,
            _ => self,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }

    pub fn set_fg(&mut self, c: Color) {
        self.0.set_bits(0..=3, c as u8);
    }

    pub fn set_bg(&mut self, c: Color) {
        self.0.set_bits(4..=7, c as u8);
    }

    pub fn fg(&self) -> Color {
        unsafe { core::mem::transmute::<u8, Color>(self.0.get_bits(0..=3)) }
    }

    pub fn bg(&self) -> Color {
        unsafe { core::mem::transmute::<u8, Color>(self.0.get_bits(4..=7)) }
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

    pub fn char(&self) -> u8 {
        self.char
    }

    pub fn fg(&self) -> Color {
        self.color.fg()
    }

    pub fn bg(&self) -> Color {
        self.color.bg()
    }
}

pub trait VideoDriver: Sync + Send {
    fn write_str(&self, _s: &str) -> Result<(), Error> {
        Ok(())
    }
    fn update_cursor(&self, _x: usize, _y: usize) {}
    fn clear(&self) {}
    fn set_cursor_visible(&self, _vis: bool) {}

    fn dimensions(&self) -> (usize, usize) {
        (0, 0)
    }
    fn copy_txt_buffer(&self, _x: usize, _y: usize, _buf: &[ScreenChar]) {}

    fn init_dev(&self) {}
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

pub fn init(fb_info: Option<&'static FramebufferInfo>) {
    crate::arch::dev::serial::init();
    if let Some(fb) = fb_info {
        if fb.typ() == 2 {
            crate::drivers::video::vga::init();
        } else {
            crate::drivers::video::fb::init(fb);
        }
    }
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

pub fn log_fmt_disabled(_args: core::fmt::Arguments) -> core::fmt::Result {
    //core::fmt::write(&mut Log {}, args)
    Ok(())
}

#[macro_export]
macro_rules! println {
    ($fmt:expr_2021) => (print!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::arch::output::write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! logln_disabled {
    ($fmt:expr_2021) => (log_disabled!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log_disabled!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log_disabled {
    ($($arg:tt)*) => {{
        let _ = ($($arg)*);
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! logln {
    ($fmt:expr_2021) => (log!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log {
    ($($arg:tt)*) => ({
        crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    });
}
#[macro_export]
macro_rules! logln2 {
    ($fmt:expr_2021) => (log2!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log2!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log2 {
    ($($arg:tt)*) => {{
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}
#[macro_export]
macro_rules! logln3 {
    ($fmt:expr_2021) => (log3!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log3!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log3 {
    ($($arg:tt)*) => {{
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! logln4 {
    ($fmt:expr_2021) => (log4!(concat!("[log4]: ", $fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log4!(concat!("[log4]: ", $fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log4 {
    ($($arg:tt)*) => {{
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! logln5 {
    ($fmt:expr_2021) => (log5!(concat!("[log5]: ", $fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log5!(concat!("[log5]: ", $fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log5 {
    ($($arg:tt)*) => {{
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! logln6 {
    ($fmt:expr_2021) => (log6!(concat!($fmt, "\n")));
    ($fmt:expr_2021, $($arg:tt)*) => (log6!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
#[allow(unused)]
macro_rules! log6 {
    ($($arg:tt)*) => {{
        $crate::arch::output::log_fmt_disabled(format_args!($($arg)*)).unwrap();
    }};
}
