use core::ptr::Unique;

use kernel::mm::MappedAddr;

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

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct Buffer {
    chars: [ScreenChar; BUFFER_WIDTH * BUFFER_HEIGHT],
}

pub struct Writer {
    column: usize,
    row: usize,
    color: ColorCode,
    buffer: Unique<Buffer>,
}

fn mk_scr_char(c: u8, clr: ColorCode) -> ScreenChar {
    ScreenChar {
        char: c,
        color: clr,
    }
}

impl Writer {
    pub fn new(fg: Color, bg: Color, buf: MappedAddr) -> Writer {
        Writer {
            column: 0,
            row: 0,
            color: ColorCode::new(fg, bg),
            buffer: unsafe {Unique::new_unchecked(buf.0 as *mut _ )},
        }
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

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
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

    pub fn clear(&mut self) {
        let blank = mk_scr_char(b' ', self.color);

        for i in 0..(BUFFER_HEIGHT * BUFFER_WIDTH) {
            self.buffer().chars[i] = blank;
        }
    }

    #[allow(unused)]
    fn clear_row(&mut self) {
        let blank = mk_scr_char(b' ', self.color);
        let row = self.row;

        for i in (row * BUFFER_WIDTH)..(row * BUFFER_WIDTH + BUFFER_WIDTH) {
            self.buffer().chars[i] = blank;
        }
    }

    #[allow(unused)]
    pub fn write_str(&mut self, s: &str)  -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        self.scroll();
        Ok(())
    }
}

impl ::core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}
