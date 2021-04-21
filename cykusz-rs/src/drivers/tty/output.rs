use alloc::vec::Vec;

use crate::arch::output::{video, Color, ColorCode, ScreenChar};

use crate::kernel::utils::types::Align;

pub struct OutputBuffer {
    buffer: Vec<ScreenChar>,

    color: ColorCode,

    viewport_y: usize,
    buffer_start_y: usize,

    line_count: usize,
    buffer_lines: usize,

    cursor_x: usize,
    cursor_y: usize,

    size_x: usize,
    size_y: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum OutputUpdate {
    None,
    Line(usize, usize), // (x, num)
    Viewport,
}

impl OutputUpdate {
    fn update_line(&mut self, line: usize) {
        match self {
            OutputUpdate::None => *self = OutputUpdate::Line(line, 1),
            OutputUpdate::Line(cur, len) => {
                if line < *cur {
                    let offset = *cur - line;

                    *cur = line;
                    *len += offset;
                } else if *cur + *len < line {
                    let offset = line - *cur + *len;

                    *len += offset;
                }
            }
            _ => {}
        }
    }

    fn update_viewport(&mut self) {
        *self = OutputUpdate::Viewport;
    }
}

impl OutputBuffer {
    fn blank(&self) -> ScreenChar {
        ScreenChar::new(b' ', self.color)
    }

    pub fn new(size_x: usize, size_y: usize, backlog: usize, fg: Color, bg: Color) -> OutputBuffer {
        let y = core::cmp::max(size_y, backlog);

        let buf_size = size_x * y;

        let mut buffer = Vec::<ScreenChar>::with_capacity(buf_size);
        buffer.resize(buf_size, ScreenChar::new(b' ', ColorCode::new(fg, bg)));

        OutputBuffer {
            buffer,

            color: ColorCode::new(fg, bg),

            viewport_y: 0,
            buffer_start_y: 0,

            line_count: 0,
            buffer_lines: backlog,

            cursor_x: 0,
            cursor_y: 0,

            size_x,
            size_y,
        }
    }

    fn cursor_pos(&self) -> usize {
        self.cursor_x + self.cursor_y * self.size_x
    }

    fn cursor_buf_pos(&self) -> usize {
        let buf_pos = self.viewport_y * self.size_x + self.cursor_pos();

        buf_pos % self.buffer.len()
    }

    fn new_line(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        let blank = self.blank();

        if self.cursor_y >= self.size_y {
            self.get_buffer_line_mut(self.cursor_y).fill(blank);
        }
    }

    fn set_viewport_line(&mut self, line: usize) {
        assert!(line < self.buffer_lines);

        self.viewport_y = (self.buffer_start_y + line) % self.buffer_lines;
    }

    fn inc_viewport_y(&mut self, by: usize) {
        self.viewport_y = (self.viewport_y + by) % self.buffer_lines;
    }

    fn inc_buffer_start_y(&mut self, by: usize) {
        self.buffer_start_y = (self.buffer_start_y + by) % self.buffer_lines;
    }

    fn store_char(&mut self, char: u8, update: &mut OutputUpdate) {
        log!("{}", char as char);

        match char {
            b'\n' => {
                self.new_line();
            }
            c => {
                let pos = self.cursor_buf_pos();

                self.buffer[pos] = ScreenChar::new(c, self.color);

                self.cursor_x += 1;

                if self.cursor_x >= self.size_x {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                }
            }
        }

        update.update_line(self.cursor_y);
    }

    fn scroll(&mut self, update: &mut OutputUpdate) {
        self.line_count = self.current_line() + 1;

        if self.cursor_y >= self.size_y {
            let off = self.cursor_y - self.size_y + 1;

            self.cursor_y = self.size_y - 1;

            self.inc_viewport_y(off);

            update.update_viewport();
        }

        if self.line_count > self.buffer_lines {
            let amount = self.line_count - self.buffer_lines;

            self.inc_buffer_start_y(amount);

            self.line_count = self.buffer_lines;
        }
    }

    fn get_buffer_line_mut(&mut self, line: usize) -> &mut [ScreenChar] {
        let buf = (self.viewport_y * self.size_x + line * self.size_x) % self.buffer.len();

        &mut self.buffer[buf..buf + self.size_x]
    }

    fn get_buffer_line(&self, line: usize) -> &[ScreenChar] {
        let buf = (self.viewport_y * self.size_x + line * self.size_x) % self.buffer.len();

        &self.buffer[buf..buf + self.size_x]
    }

    fn viewport_buf_start(&self) -> usize {
        self.viewport_y * self.size_x
    }

    fn viewport_buf_end(&self) -> usize {
        (self.viewport_buf_start() + self.size_x * self.size_y) % self.buffer.len()
    }

    fn update_screen(&mut self, update: OutputUpdate) {
        let video = video();

        match update {
            OutputUpdate::None => {}
            OutputUpdate::Line(line, num) => {
                for l in line..line + num {
                    video.copy_txt_buffer(0, l, self.get_buffer_line(l));
                }
            }
            OutputUpdate::Viewport => {
                let viewport_start = self.viewport_buf_start();
                let viewport_end = self.viewport_buf_end();

                if viewport_start < viewport_end {
                    video.copy_txt_buffer(0, 0, &self.buffer[viewport_start..viewport_end]);
                } else {
                    let split_lines = self.buffer_lines - self.viewport_y;

                    video.copy_txt_buffer(0, 0, &self.buffer[viewport_start..]);
                    video.copy_txt_buffer(0, split_lines, &self.buffer[..viewport_end]);
                }
            }
        }

        video.update_cursor(self.cursor_x, self.cursor_y);
    }

    fn current_line(&self) -> usize {
        self.viewport_line() + self.cursor_y
    }

    fn viewport_line(&self) -> usize {
        if self.viewport_y >= self.buffer_start_y {
            self.viewport_y - self.buffer_start_y
        } else {
            (self.buffer_lines - self.buffer_start_y) + self.viewport_y
        }
    }

    fn max_line(&self) -> usize {
        if self.line_count > self.size_y {
            self.line_count - self.size_y
        } else {
            0
        }
    }

    fn _scroll_up(&mut self, lines: usize) -> bool {
        let current_line = self.viewport_line();

        let target_line = if lines > current_line {
            0
        } else {
            current_line - lines
        };

        if current_line != target_line {
            self.set_viewport_line(target_line);

            true
        } else {
            false
        }
    }

    fn _scroll_down(&mut self, lines: usize) -> bool {
        let max_line = self.max_line();

        let mut current_line = self.viewport_line();

        if current_line < max_line {
            current_line = core::cmp::min(current_line + lines, max_line);

            self.set_viewport_line(current_line);

            true
        } else {
            false
        }
    }

    fn _scroll_bottom(&mut self) -> bool {
        let max_line = self.max_line();

        let current_line = self.viewport_line();

        if current_line < max_line {
            self.set_viewport_line(max_line);

            true
        } else {
            false
        }
    }

    fn _scroll_top(&mut self) -> bool {
        if self.viewport_line() > 0 {
            self.set_viewport_line(0);

            true
        } else {
            false
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        if self._scroll_up(lines) {
            self.update_screen(OutputUpdate::Viewport);
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        if self._scroll_down(lines) {
            self.update_screen(OutputUpdate::Viewport);
        }
    }

    pub fn scroll_bottom(&mut self) {
        if self._scroll_bottom() {
            self.update_screen(OutputUpdate::Viewport);
        }
    }

    pub fn scroll_top(&mut self) {
        if self._scroll_top() {
            self.update_screen(OutputUpdate::Viewport);
        }
    }

    pub fn put_char(&mut self, char: u8) {
        let mut update = OutputUpdate::None;

        if self._scroll_bottom() {
            update.update_viewport();
        }

        self.store_char(char, &mut update);

        self.scroll(&mut update);

        self.update_screen(update);
    }

    pub fn write_str(&mut self, str: &str) {
        let mut update = OutputUpdate::None;

        if self._scroll_bottom() {
            update.update_viewport();
        }

        let mut performer = AnsiEscape::new(self, update);
        let mut state = vte::Parser::new();

        let bytes = str.as_bytes();

        for c in bytes.iter() {
            state.advance(&mut performer, *c);
        }

        let mut update = performer.update_delta();

        self.scroll(&mut update);

        self.update_screen(update);
    }

    pub fn remove_last_n(&mut self, mut n: usize) {
        let blank = self.blank();

        let mut update = OutputUpdate::None;

        if self._scroll_bottom() {
            update.update_viewport();
        }

        while n > 0 && (self.cursor_x != 0 || self.cursor_y != 0) {
            let pos = self.cursor_buf_pos() - 1;

            self.buffer[pos] = blank;

            if self.cursor_x == 0 {
                self.cursor_x = self.size_x - 1;
                self.cursor_y -= 1;
            } else {
                self.cursor_x -= 1;
            }

            update.update_line(self.cursor_y);

            n -= 1;
        }

        self.update_screen(update);
    }
}

struct AnsiEscape<'a> {
    output: &'a mut OutputBuffer,
    update: OutputUpdate,
}

impl<'a> AnsiEscape<'a> {
    fn new(output: &'a mut OutputBuffer, update: OutputUpdate) -> AnsiEscape<'a> {
        AnsiEscape::<'a> { output, update }
    }

    fn update_delta(&self) -> OutputUpdate {
        self.update
    }
}

enum ParsedColor {
    Unknown,
    Foreground(Color),
    Background(Color),
}

fn to_color(code: u16) -> ParsedColor {
    match code {
        30 => return ParsedColor::Foreground(Color::Black),
        31 => return ParsedColor::Foreground(Color::Red),
        32 => return ParsedColor::Foreground(Color::Green),
        33 => return ParsedColor::Foreground(Color::Brown),
        34 => return ParsedColor::Foreground(Color::Blue),
        35 => return ParsedColor::Foreground(Color::Magenta),
        36 => return ParsedColor::Foreground(Color::Cyan),
        37 => return ParsedColor::Foreground(Color::LightGray),
        40 => return ParsedColor::Background(Color::Black),
        41 => return ParsedColor::Background(Color::Red),
        42 => return ParsedColor::Background(Color::Green),
        43 => return ParsedColor::Background(Color::Brown),
        44 => return ParsedColor::Background(Color::Blue),
        45 => return ParsedColor::Background(Color::Magenta),
        46 => return ParsedColor::Background(Color::Cyan),
        47 => return ParsedColor::Background(Color::LightGray),
        90 => return ParsedColor::Foreground(Color::DarkGray),
        91 => return ParsedColor::Foreground(Color::LightRed),
        92 => return ParsedColor::Foreground(Color::LightGreen),
        93 => return ParsedColor::Foreground(Color::Yellow),
        94 => return ParsedColor::Foreground(Color::LightBlue),
        95 => return ParsedColor::Foreground(Color::Pink),
        96 => return ParsedColor::Foreground(Color::LightCyan),
        97 => return ParsedColor::Foreground(Color::White),
        100 => return ParsedColor::Background(Color::DarkGray),
        101 => return ParsedColor::Background(Color::LightRed),
        102 => return ParsedColor::Background(Color::LightGreen),
        103 => return ParsedColor::Background(Color::Yellow),
        104 => return ParsedColor::Background(Color::LightBlue),
        105 => return ParsedColor::Background(Color::Pink),
        106 => return ParsedColor::Background(Color::LightCyan),
        107 => return ParsedColor::Background(Color::White),
        _ => {}
    };

    ParsedColor::Unknown
}

impl<'a> vte::Perform for AnsiEscape<'a> {
    fn print(&mut self, c: char) {
        self.output.store_char(c as u8, &mut self.update);
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\n' {
            self.output.store_char(byte, &mut self.update);
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        use core::cmp::min;

        if ignore {
            return;
        }

        match action {
            'H' | 'f' => {
                let mut iter = params.iter();
                let y = iter.next().unwrap_or(&[1u16])[0] as usize;
                let x = iter.next().unwrap_or(&[1u16])[0] as usize;
                let y = if y != 0 { y - 1} else { y };
                let x = if x != 0 { x - 1} else { x };
                self.output.cursor_y = min(y, self.output.size_y - 1);
                self.output.cursor_x = min(x, self.output.size_x - 1);
            }
            'A' | 'F' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    if self.output.cursor_y >= x as usize {
                        self.output.cursor_y -= x as usize;
                    } else {
                        self.output.cursor_y = 0;
                    }

                    if action == 'F' {
                        self.output.cursor_x = 0;
                    }
                }
            }
            'B' | 'E' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    let cur = self.output.cursor_y;

                    self.output.cursor_y = min(self.output.size_y - 1, cur + x as usize);

                    if action == 'E' {
                        self.output.cursor_x = 0;
                    }
                }
            }
            'C' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    let cur = self.output.cursor_x;

                    self.output.cursor_x = min(self.output.size_x - 1, cur + x as usize);
                }
            }
            'D' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    if self.output.cursor_x >= x as usize {
                        self.output.cursor_x -= x as usize;
                    } else {
                        self.output.cursor_x = 0;
                    }
                }
            }
            'G' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    self.output.cursor_x = min(x as usize, self.output.size_x - 1);
                }
            }
            'm' => {
                let iter = params.iter();

                let mut bright = false;
                let mut dim = false;

                for m in iter {
                    if !m.is_empty() {
                        let m = m[0];

                        match m {
                            0 => {
                                bright = false;
                                dim = false;
                                self.output.color = ColorCode::new(Color::LightGreen, Color::Black);
                            }
                            1 => {
                                bright = true;
                                dim = false;
                            }
                            2 => {
                                dim = true;
                                bright = false;
                            }
                            m => match to_color(m) {
                                ParsedColor::Background(c) => {
                                    self.output.color.set_bg(c);
                                }
                                ParsedColor::Foreground(mut c) => {
                                    if dim {
                                        c = c.dim();
                                    } else if bright {
                                        c = c.brighten();
                                    }

                                    self.output.color.set_fg(c);
                                }
                                ParsedColor::Unknown => {}
                            },
                        }
                    }
                }
            }
            'K' => {
                let mut iter = params.iter();
                if let Some(&[x, ..]) = iter.next() {
                    let buf_pos = self.output.cursor_buf_pos();

                    let blank = self.output.blank();

                    match x {
                        0 => {
                            let end = buf_pos.align_up(self.output.size_x);

                            self.output.buffer[buf_pos..end].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        },
                        1 => {
                            let beg = buf_pos.align_down(self.output.size_x);

                            self.output.buffer[beg..=buf_pos].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        },
                        2 => {
                            let beg = buf_pos.align_down(self.output.size_x);
                            let end = buf_pos.align_up(self.output.size_x);

                            self.output.buffer[beg..end].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        },
                        _ => {}
                    }
                }
            }
            'J' => {
                let mut iter = params.iter();
                if let Some(&[x, ..]) = iter.next() {
                    let v_buf_start = self.output.viewport_buf_start();
                    let v_buf_end = self.output.viewport_buf_end();
                    let c_buf_pos = self.output.cursor_buf_pos();

                    if let Some((clear_start, clear_end)) = match x {
                        0 => {
                            Some((c_buf_pos, v_buf_end))
                        },
                        1 => {
                            Some((v_buf_start, c_buf_pos + 1))
                        },
                        2 => {
                            Some((v_buf_start, v_buf_end))
                        },
                        _ => {
                            None
                        }
                    } {
                        let blank = self.output.blank();

                        if clear_start < clear_end {
                            self.output.buffer[clear_start..clear_end].fill(blank);
                        } else {
                            self.output.buffer[clear_start..].fill(blank);
                            self.output.buffer[..clear_end].fill(blank);
                        }

                        self.update.update_viewport();
                    }
                }

            }
            _ => {}
        }
    }
}
