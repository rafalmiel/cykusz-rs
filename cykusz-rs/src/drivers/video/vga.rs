use core::ptr::Unique;

use bit_field::BitField;

use crate::arch::mm::PhysAddr;
use crate::arch::output::{Character, VideoDriver};
use crate::arch::raw::cpuio::Port;
use crate::drivers::tty::color::{ColorCode as AnsiColorCode, Ansi16};
use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::{LockApi, Spin};

const VGA_BUFFER: MappedAddr = PhysAddr(0xb8000).to_mapped();

static CURSOR_INDEX: Spin<Port<u8>> = Spin::new(unsafe { Port::new(0x3D4) });
static CURSOR_DATA: Spin<Port<u8>> = Spin::new(unsafe { Port::new(0x3D5) });

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

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ColorCode(u8);

const fn map_to_vga(color: Ansi16) -> u8 {
    match color.color() {
        AnsiColorCode::Black => { 0 }
        AnsiColorCode::Red => { 4 }
        AnsiColorCode::Green => { 2 }
        AnsiColorCode::Yellow => { 6 }
        AnsiColorCode::Blue => { 1 }
        AnsiColorCode::Magenta => { 5 }
        AnsiColorCode::Cyan => { 3 }
        AnsiColorCode::White => { 7 }
        AnsiColorCode::LightBlack => { 8 }
        AnsiColorCode::LightRed => { 12 }
        AnsiColorCode::LightGreen => { 10 }
        AnsiColorCode::LightYellow => { 14 }
        AnsiColorCode::LightBlue => { 9 }
        AnsiColorCode::LightMagenta => { 13 }
        AnsiColorCode::LightCyan => { 11 }
        AnsiColorCode::LightWhite => { 15 }
    }
}


impl ColorCode {
    pub const fn new(foreground: Ansi16, background: Ansi16) -> ColorCode {
        ColorCode(map_to_vga(background) << 4 | map_to_vga(foreground))
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

impl From<Character> for ScreenChar {
    fn from(value: Character) -> Self {
        ScreenChar {
            char: value.character(),
            color: ColorCode::new(value.foreground().into(), value.background().into())
        }
    }
}

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
    state: Spin<State>,
}

fn mk_scr_char(c: u8, clr: ColorCode) -> ScreenChar {
    ScreenChar::new(c, clr)
}

impl State {
    pub const fn new(fg: Ansi16, bg: Ansi16, buf: MappedAddr) -> State {
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

        self.buffer().chars.fill(blank);

        update_cursor(self.buffer_pos());
    }
}

impl Writer {
    pub const fn new(fg: Ansi16, bg: Ansi16, buf: MappedAddr) -> Writer {
        Writer {
            state: Spin::new(State::new(fg, bg, buf)),
        }
    }
}

impl VideoDriver for Writer {
    fn write_str(&self, s: &str) -> ::core::fmt::Result {
        self.state.lock().write_str(s)
    }

    fn update_cursor(&self, x: usize, y: usize) {
        update_cursor((x + y * BUFFER_WIDTH) as u16)
    }

    fn clear(&self) {
        self.state.lock().clear()
    }

    fn set_cursor_visible(&self, vis: bool) {
        set_cursor_vis(vis);
    }

    fn dimensions(&self) -> (usize, usize) {
        (BUFFER_WIDTH, BUFFER_HEIGHT)
    }

    fn copy_txt_buffer(&self, x: usize, y: usize, buf: &[Character]) {
        if x >= BUFFER_WIDTH || y >= BUFFER_HEIGHT {
            return;
        }

        let offset = y * BUFFER_WIDTH + x;
        let len = core::cmp::min(BUFFER_HEIGHT * BUFFER_WIDTH - offset, buf.len());

        let mut state = self.state.lock();
        let chars = unsafe {
            &mut state.buffer.as_mut().chars
        };
        for (buf_i, dest_i) in (offset..offset + len).enumerate() {
            chars[dest_i] = buf[buf_i].into();
        }
    }
}

static VGA: Writer = Writer::new(Ansi16::new(AnsiColorCode::LightGreen),
                                 Ansi16::new(AnsiColorCode::Black), VGA_BUFFER);

// References:
// - http://www.osdever.net/FreeVGA/vga/attrreg.htm#10
// - http://www.osdever.net/FreeVGA/vga/vgareg.htm#attribute
fn disable_text_blink() {
    let mut input_status = unsafe { Port::<u8>::new(0x3da) };

    let mut attr_reg = unsafe { Port::<u8>::new(0x3c0) };

    input_status.read();
    let attr = attr_reg.read();
    attr_reg.write(0x10 | 0b100000);
    let mut val = attr_reg.read();
    val.set_bit(3, false);
    attr_reg.write(val);
    attr_reg.write(attr);
}

fn set_cursor_vis(vis: bool) {
    let idx = &mut *CURSOR_INDEX.lock_irq();
    let dta = &mut *CURSOR_DATA.lock_irq();

    idx.write(0x0A);

    let mut dat = dta.read();
    dat.set_bit(5, !vis);

    idx.write(0x0A);
    dta.write(dat);
}

pub fn init() {
    disable_text_blink();

    crate::arch::output::register_video_driver(&VGA);
}
