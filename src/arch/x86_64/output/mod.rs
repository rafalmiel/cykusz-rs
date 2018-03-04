use ::drivers::video::vga::{Color, Writer};
use arch::cpuio::Port;
use arch::types::MappedAddr;
use spin::Mutex;

const VGA_BUFFER: MappedAddr = MappedAddr(0xffff8000000b8000);

static CURSOR_INDEX: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x3D4) });
static CURSOR_DATA: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x3D5) });

fn update_cursor(offset: u16)
{
    CURSOR_INDEX.lock().write(0x0F);
    CURSOR_DATA.lock().write((offset & 0xFF) as u8);

    CURSOR_INDEX.lock().write(0x0E);
    CURSOR_DATA.lock().write((offset >> 8) as u8);
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> =
        Mutex::new(
            Writer::new(Color::LightGreen, Color::Black, VGA_BUFFER)
        );
}

pub fn clear() {
    let w = &mut *WRITER.lock();
    w.clear();
    update_cursor(w.buffer_pos());
}

pub fn write_fmt(args: ::core::fmt::Arguments) -> ::core::fmt::Result {
    let mut w = &mut *WRITER.lock();
    let r = ::core::fmt::write(&mut w, args);
    update_cursor(w.buffer_pos());
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
        $crate::arch::output::write_fmt(format_args!($($arg)*)).unwrap();
    });
}