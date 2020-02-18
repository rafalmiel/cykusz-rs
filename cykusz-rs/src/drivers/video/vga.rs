use core::ptr::Unique;

use crate::arch::output::ConsoleDriver;
use crate::arch::raw::cpuio::Port;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::Mutex;

#[allow(unused)]
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
struct ScreenChar {
    char: u8,
    color: ColorCode,
}

const VGA_BUFFER: MappedAddr = MappedAddr(0xffff8000000b8000);

static CURSOR_INDEX: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x3D4) });
static CURSOR_DATA: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x3D5) });

fn update_cursor(offset: u16) {
    let idx = &mut *CURSOR_INDEX.lock_irq();
    let dta = &mut *CURSOR_DATA.lock_irq();

    idx.write(0x0F);
    dta.write((offset & 0xFF) as u8);

    idx.write(0x0E);
    dta.write((offset >> 8) as u8);
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct Buffer {
    chars: [ScreenChar; BUFFER_WIDTH * BUFFER_HEIGHT],
}

struct State {
    column: usize,
    row: usize,
    color: ColorCode,
    buffer: Unique<Buffer>,
}

pub struct Writer {
    state: Mutex<State>,
}

fn mk_scr_char(c: u8, clr: ColorCode) -> ScreenChar {
    ScreenChar {
        char: c,
        color: clr,
    }
}

impl State {
    pub const fn new(fg: Color, bg: Color, buf: MappedAddr) -> State {
        State {
            column: 0,
            row: 0,
            color: ColorCode::new(fg, bg),
            buffer: unsafe { Unique::new_unchecked(buf.0 as *mut _) },
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let row = self.row;
                let col = self.column;

                self.buffer().chars[row * BUFFER_WIDTH + col] = mk_scr_char(byte, self.color);
                self.column += 1;

                if self.column == 80 {
                    self.column = 0;
                    self.row += 1;
                }
            }
        }

        self.scroll();
    }

    pub fn buffer_pos(&self) -> u16 {
        (BUFFER_WIDTH * self.row + self.column) as u16
    }

    fn scroll(&mut self) {
        if self.row > BUFFER_HEIGHT - 1 {
            let blank = mk_scr_char(b' ', self.color);

            {
                let buffer = self.buffer();
                for i in 0..((BUFFER_HEIGHT - 1) * (BUFFER_WIDTH)) {
                    buffer.chars[i] = buffer.chars[i + BUFFER_WIDTH];
                }

                for i in ((BUFFER_HEIGHT - 1) * (BUFFER_WIDTH))..(BUFFER_HEIGHT * BUFFER_WIDTH) {
                    buffer.chars[i] = blank;
                }
            }

            self.row = BUFFER_HEIGHT - 1;
        }
    }

    fn new_line(&mut self) {
        self.column = 0;
        self.row += 1;
    }

    #[allow(unused)]
    fn clear_row(&mut self) {
        let blank = mk_scr_char(b' ', self.color);
        let row = self.row;

        for i in (row * BUFFER_WIDTH)..(row * BUFFER_WIDTH + BUFFER_WIDTH) {
            self.buffer().chars[i] = blank;
        }
    }

    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        self.scroll();
        update_cursor(self.buffer_pos());
        Ok(())
    }

    fn clear(&mut self) {
        let blank = mk_scr_char(b' ', self.color);

        for i in 0..(BUFFER_HEIGHT * BUFFER_WIDTH) {
            self.buffer().chars[i] = blank;
        }
        update_cursor(self.buffer_pos());
    }

    fn remove_last_n(&mut self, mut n: usize) {
        let blank = mk_scr_char(b' ', self.color);
        while n > 0 {
            let pos = self.buffer_pos();
            if pos == 0 {
                return;
            }
            self.buffer().chars[pos as usize - 1] = blank;
            if self.column == 0 {
                self.column = 79;
                self.row -= 1;
            } else {
                self.column -= 1;
            }
            n -= 1;
        }
        update_cursor(self.buffer_pos());
    }
}

impl Writer {
    pub const fn new(fg: Color, bg: Color, buf: MappedAddr) -> Writer {
        Writer {
            state: Mutex::new(State::new(fg, bg, buf)),
        }
    }
}

impl ConsoleDriver for Writer {
    fn write_str(&self, s: &str) -> ::core::fmt::Result {
        crate::arch::dev::serial::write(s);
        self.state.lock().write_str(s)
    }

    fn clear(&self) {
        self.state.lock().clear()
    }

    fn remove_last_n(&self, n: usize) {
        self.state.lock().remove_last_n(n)
    }
}

static WRITER: Writer = Writer::new(Color::LightGreen, Color::Black, VGA_BUFFER);

pub fn init() {
    crate::arch::output::register_console_driver(&WRITER);
}
