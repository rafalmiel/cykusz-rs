use core::ptr::Unique;

use bit_field::BitField;

use crate::arch::mm::PhysAddr;
use crate::arch::output::{Color, ColorCode, ScreenChar, VideoDriver};
use crate::arch::raw::cpuio::Port;
use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::Spin;

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

        self.buffer().chars.fill(blank);

        update_cursor(self.buffer_pos());
    }
}

impl Writer {
    pub const fn new(fg: Color, bg: Color, buf: MappedAddr) -> Writer {
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

    fn copy_txt_buffer(&self, x: usize, y: usize, buf: &[ScreenChar]) {
        if x >= BUFFER_WIDTH || y >= BUFFER_HEIGHT {
            return;
        }

        let offset = y * BUFFER_WIDTH + x;
        let len = core::cmp::min(BUFFER_HEIGHT * BUFFER_WIDTH - offset, buf.len());

        let mut state = self.state.lock();
        unsafe {
            state.buffer.as_mut().chars[offset..offset + len].copy_from_slice(buf);
        }
    }
}

static VGA: Writer = Writer::new(Color::LightGreen, Color::Black, VGA_BUFFER);

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
