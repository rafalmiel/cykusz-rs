use alloc::vec::Vec;
use core::cmp::max;
use core::ops::Range;

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

    saved_x: usize,
    saved_y: usize,

    size_x: usize,
    size_y: usize,

    state: vte::Parser,
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
                } else if *cur + *len - 1 < line {
                    let offset = line - *cur + *len - 1;

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

struct BufRangeIter {
    size: usize,
    start: usize,
    end: usize,
    index: usize,
}

impl BufRangeIter {
    fn new(size: usize, start: usize, end: usize) -> BufRangeIter {
        BufRangeIter {
            size,
            start,
            end,
            index: 0,
        }
    }
}

impl Iterator for BufRangeIter {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if self.start < self.end && self.index == 1 {
            Some(self.start..self.end)
        } else if self.start > self.end && self.index < 3 {
            match self.index {
                1 => Some(self.start..self.size),
                2 => Some(0..self.end),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for BufRangeIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if self.start < self.end && self.index == 1 {
            Some(self.start..self.end)
        } else if self.start > self.end && self.index < 3 {
            match self.index {
                1 => Some(0..self.end),
                2 => Some(self.start..self.size),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }
}

impl OutputBuffer {
    fn blank(&self) -> ScreenChar {
        ScreenChar::new(b' ', self.color)
    }

    pub fn new(size_x: usize, size_y: usize, backlog: usize, fg: Color, bg: Color) -> OutputBuffer {
        let y = max(size_y, backlog);

        let buf_size = size_x * y;

        let mut buffer = Vec::<ScreenChar>::with_capacity(buf_size);
        buffer.resize(buf_size, ScreenChar::new(b' ', ColorCode::new(fg, bg)));

        OutputBuffer {
            buffer,

            color: ColorCode::new(fg, bg),

            viewport_y: 0,
            buffer_start_y: 0,

            line_count: 0,
            buffer_lines: y,

            cursor_x: 0,
            cursor_y: 0,

            saved_x: 0,
            saved_y: 0,

            size_x,
            size_y,

            state: vte::Parser::new(),
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

        if self.cursor_y >= self.size_y {
            let blank = self.blank();

            self.get_buffer_line_mut(self.cursor_y).fill(blank);
        }
    }

    fn tab(&mut self) {
        self.cursor_x = (self.cursor_x + 8).align_down(8);
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
        match char {
            b'\n' => {
                self.new_line();
            }
            b'\t' => {
                self.tab();

                if self.cursor_x >= self.size_x {
                    self.new_line();
                }
            }
            b'\r' => {
                self.cursor_x = 0;
            }
            c => {
                if self.cursor_x >= self.size_x {
                    self.new_line();
                }

                let pos = self.cursor_buf_pos();

                self.buffer[pos] = ScreenChar::new(c, self.color);

                self.cursor_x += 1;
            }
        }

        self.line_count = core::cmp::max(self.line_count, self.current_line() + 1);

        update.update_line(self.cursor_y);
    }

    fn scroll(&mut self, update: &mut OutputUpdate) {
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

    fn get_buffer_line_pos(&self, line: usize) -> usize {
        (self.viewport_y * self.size_x + line * self.size_x) % self.buffer.len()
    }

    fn get_buffer_line_end_pos(&self, line: usize) -> usize {
        let mut pos = self.viewport_y * self.size_x + line * self.size_x;

        if pos > self.buffer.len() {
            pos -= self.buffer.len();
        }

        pos
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
        let end = self.viewport_buf_start() + self.size_x * self.size_y;

        if end > self.buffer.len() {
            return end - self.buffer.len();
        }

        end
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

    fn clear_lines(&mut self, line_start: usize, line_end: usize) {
        let blank = self.blank();

        let pos_start = self.get_buffer_line_pos(line_start);
        let pos_end = self.get_buffer_line_end_pos(line_end);

        if pos_start < pos_end {
            self.buffer[pos_start..pos_end].fill(blank);
        }
    }

    fn copy_lines_within(
        &mut self,
        cursor_start_line: usize,
        cursor_end_line: usize,
        cursor_dest_line: usize,
    ) {
        //logln4!(
        //    "Copy lines {} - {} -> {}",
        //    cursor_start_line,
        //    cursor_end_line,
        //    cursor_dest_line
        //);
        if cursor_start_line == cursor_end_line {
            return;
        }
        assert!(cursor_end_line > cursor_start_line);
        assert!(cursor_dest_line + (cursor_end_line - cursor_start_line) <= self.size_y);

        let start_pos = self.get_buffer_line_pos(cursor_start_line);
        let end_pos = self.get_buffer_line_end_pos(cursor_end_line);
        let remaining_lines = cursor_end_line - cursor_start_line;
        let start_dest_pos = self.get_buffer_line_pos(cursor_dest_line);
        let end_dest_pos = self.get_buffer_line_end_pos(cursor_dest_line + remaining_lines);

        if cursor_dest_line < cursor_start_line {
            // Copy from start
            let mut source_iter = BufRangeIter::new(self.buffer.len(), start_pos, end_pos);
            let mut dest_iter = BufRangeIter::new(self.buffer.len(), start_dest_pos, end_dest_pos);

            let mut source_range = source_iter.next();
            let mut dest_range = dest_iter.next();

            while let (Some(s), Some(d)) = (source_range.clone(), dest_range.clone()) {
                if s.len() < d.len() {
                    self.buffer.copy_within(s.clone(), d.start);
                    dest_range = Some(d.start + s.len()..d.end);
                    source_range = source_iter.next();
                } else if s.len() > d.len() {
                    self.buffer.copy_within(s.start..s.start + d.len(), d.start);
                    source_range = Some(s.start + d.len()..s.end);
                    dest_range = dest_iter.next();
                } else {
                    self.buffer.copy_within(s.clone(), d.start);
                    source_range = source_iter.next();
                    dest_range = dest_iter.next();
                }
            }
        } else if cursor_dest_line > cursor_start_line {
            // Copy from end
            let mut source_iter = BufRangeIter::new(self.buffer.len(), start_pos, end_pos).rev();
            let mut dest_iter =
                BufRangeIter::new(self.buffer.len(), start_dest_pos, end_dest_pos).rev();

            let mut source_range = source_iter.next();
            let mut dest_range = dest_iter.next();

            while let (Some(s), Some(d)) = (source_range.clone(), dest_range.clone()) {
                if s.len() < d.len() {
                    self.buffer.copy_within(s.clone(), d.end - s.len());
                    dest_range = Some(d.start..d.end - s.len());
                    source_range = source_iter.next();
                } else if s.len() > d.len() {
                    self.buffer.copy_within(s.end - d.len()..s.end, d.start);
                    source_range = Some(s.start..s.end - d.len());
                    dest_range = dest_iter.next();
                } else {
                    self.buffer.copy_within(s.clone(), d.start);
                    source_range = source_iter.next();
                    dest_range = dest_iter.next();
                }
            }
        }
    }

    pub fn shift_up(&mut self, lines: usize) {
        self.copy_lines_within(self.cursor_y, self.size_y - lines, self.cursor_y + lines);
        self.clear_lines(self.cursor_y, self.cursor_y + lines);
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

        let me = unsafe { &mut *(self as *mut OutputBuffer) };

        let mut performer = AnsiEscape::new(me, update);

        self.state.advance(&mut performer, char);

        let mut update = performer.update_delta();

        self.scroll(&mut update);

        self.update_screen(update);
    }

    pub fn write_str(&mut self, str: &str) {
        let mut update = OutputUpdate::None;

        if self._scroll_bottom() {
            update.update_viewport();
        }

        let me = unsafe { &mut *(self as *mut OutputBuffer) };

        let mut performer = AnsiEscape::new(me, update);

        let bytes = str.as_bytes();

        //log4!("[tty in]: ");
        //for b in bytes {
        //    let c = *b as char;
        //    if c.is_ascii_alphabetic() || c.is_ascii_punctuation() {
        //        log4!("{}", c);
        //    } else {
        //        log4!("{}", *b);
        //    }
        //}
        //log4!("\n");

        for c in bytes.iter() {
            self.state.advance(&mut performer, *c);
        }

        let mut update = performer.update_delta();

        self.scroll(&mut update);

        self.update_screen(update);

        //logln4!(
        //    "buf_y: {}, view_y: {}",
        //    self.buffer_start_y,
        //    self.viewport_y
        //);
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
        //logln4!("Execute: {}", byte);
        if byte == b'\n' {
            self.output.store_char(byte, &mut self.update);
        } else if byte == b'\t' {
            self.output.store_char(byte, &mut self.update);
        } else if byte == b'\r' {
            self.output.store_char(byte, &mut self.update);
        } else if byte == 8 {
            if self.output.cursor_x > 0 {
                self.output.cursor_x -= 1;
            }
        } else {
            logln4!("unrecognised ctrl {}", byte);
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

        //logln4!(
        //    "CSI DISPATCH: action: {}, params: {:?}, intermediates: {:?}",
        //    action,
        //    params,
        //    intermediates
        //);

        if ignore {
            return;
        }

        match action {
            'H' | 'f' => {
                let mut iter = params.iter();
                let y = iter.next().unwrap_or(&[1u16])[0] as usize;
                let x = iter.next().unwrap_or(&[1u16])[0] as usize;
                let y = if y != 0 { y - 1 } else { y };
                let x = if x != 0 { x - 1 } else { x };
                self.output.cursor_y = min(y, self.output.size_y - 1);
                self.output.cursor_x = min(x, self.output.size_x - 1);
            }
            'A' | 'F' => {
                let mut iter = params.iter();

                if let Some(&[x, ..]) = iter.next() {
                    //logln4!("Action A: x: {}", x);
                    if self.output.cursor_y >= x as usize {
                        self.output.cursor_y -= max(1, x) as usize;
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

                    self.output.cursor_x = min(self.output.size_x - 1, cur + max(1, x as usize));
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
            'd' => {
                let mut iter = params.iter();

                if let Some(&[y, ..]) = iter.next() {
                    let y = if y == 0 { y } else { y - 1 };
                    self.output.cursor_y = min(y as usize, self.output.size_x - 1);
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
                            7 => {
                                let fg = self.output.color.fg();
                                let bg = self.output.color.bg();

                                self.output.color.set_fg(bg);
                                self.output.color.set_bg(fg);
                            }
                            m => match to_color(m) {
                                ParsedColor::Background(mut c) => {
                                    if dim {
                                        c = c.dim();
                                    } else if bright {
                                        c = c.brighten();
                                    }

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
                            let mut end = buf_pos.align_up(self.output.size_x);

                            if end == buf_pos {
                                end = (buf_pos + 1).align_up(self.output.size_x);
                            }

                            self.output.buffer[buf_pos..end].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        }
                        1 => {
                            let beg = buf_pos.align_down(self.output.size_x);

                            self.output.buffer[beg..=buf_pos].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        }
                        2 => {
                            let beg = buf_pos.align_down(self.output.size_x);
                            let end = buf_pos.align_up(self.output.size_x);

                            self.output.buffer[beg..end].fill(blank);

                            self.update.update_line(self.output.cursor_y);
                        }
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
                        0 => Some((c_buf_pos, v_buf_end)),
                        1 => Some((v_buf_start, c_buf_pos + 1)),
                        2 => Some((v_buf_start, v_buf_end)),
                        _ => None,
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
            'l' | 'h' => match params.iter().next() {
                Some([25]) => {
                    video().set_cursor_visible(action == 'h');
                }
                _ => {}
            },
            'L' => {
                let mut iter = params.iter();
                if let Some(&[x, ..]) = iter.next() {
                    let lines = max(1, x);
                    self.output.shift_up(lines as usize);
                    self.update.update_viewport();
                }
            }
            's' => {
                self.output.saved_x = self.output.cursor_x;
                self.output.saved_y = self.output.cursor_y;
            }
            'u' => {
                self.output.cursor_x = self.output.saved_x;
                self.output.cursor_y = self.output.saved_y;
            }
            a => {
                panic!("Unhandled cmd {}", a);
            }
        }
    }
}
