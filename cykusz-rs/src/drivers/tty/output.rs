use alloc::vec::Vec;

use crate::arch::output::{video, Color, ColorCode, ScreenChar};

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

pub enum OutputUpdate {
    Line(usize, usize), // (x, num)
    Viewport,
}

impl OutputUpdate {
    fn inc_lines(&mut self, by: usize) {
        if let OutputUpdate::Line(_x, y) = self {
            *y = *y + by;
        }
    }

    fn move_prev(&mut self) {
        if let OutputUpdate::Line(x, y) = self {
            if *x > 0 {
                *x = *x - 1;
            }
            *y = *y + 1;
        }
    }
}

impl OutputBuffer {
    pub fn new(size_x: usize, size_y: usize, backlog: usize, fg: Color, bg: Color) -> OutputBuffer {
        let y = core::cmp::max(size_y, backlog);

        let buf_size = size_x * y;

        let mut buffer = Vec::<ScreenChar>::with_capacity(buf_size);
        buffer.resize(
            buf_size,
            ScreenChar::new(b' ', ColorCode::new(Color::LightGreen, Color::Black)),
        );

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

        self.line_count += 1;

        let line_pos = (self.buffer_start_y + self.line_count) % self.buffer_lines;

        self.buffer[line_pos * self.size_x..line_pos * self.size_x + self.size_x].fill(
            ScreenChar::new(b' ', ColorCode::new(Color::LightGreen, Color::Black)),
        );
    }

    fn store_char(&mut self, char: u8, update: &mut OutputUpdate) {
        match char {
            b'\n' => {
                self.new_line();

                update.inc_lines(1);
            }
            c => {
                let pos = self.cursor_buf_pos();

                self.buffer[pos] = ScreenChar::new(c, self.color);

                self.cursor_x += 1;
            }
        }
    }

    fn scroll(&mut self, update: &mut OutputUpdate) {
        if self.cursor_x >= self.size_x {
            let lines = self.cursor_x / self.size_x;

            self.cursor_y += lines;
            self.cursor_x = self.cursor_x % self.size_x;

            self.line_count += lines;

            update.inc_lines(lines);
        }

        if self.cursor_y >= self.size_y {
            let off = self.cursor_y - self.size_y + 1;

            self.cursor_y = self.size_y - 1;
            self.viewport_y = (off + self.viewport_y) % self.buffer_lines;

            *update = OutputUpdate::Viewport;
        }

        if self.line_count > self.buffer_lines {
            let amount = self.line_count - self.buffer_lines;

            //let old_y = self.buffer_start_y;
            self.buffer_start_y = (self.buffer_start_y + amount) % self.buffer_lines;

            //if old_y < self.buffer_start_y {
            //    let old_pos = old_y * self.size_x;

            //    self.buffer[old_pos + self.cursor_x..old_pos + amount * self.size_x].fill(
            //        ScreenChar::new(b' ', ColorCode::new(Color::LightGreen, Color::Black)),
            //    )
            //} else {
            //    //self.buffer[old_y * self.size_x..].fill(ScreenChar::new(
            //    //    0,
            //    //    ColorCode::new(Color::Black, Color::Black),
            //    //));
            //    //self.buffer[..self.buffer_start_y * self.size_x].fill(ScreenChar::new(
            //    //    0,
            //    //    ColorCode::new(Color::Black, Color::Black),
            //    //));
            //}

            self.line_count = self.buffer_lines;
        }
    }

    fn get_buffer_line(&self, line: usize) -> &[ScreenChar] {
        let buf = (self.viewport_y * self.size_x + line * self.size_x) % self.buffer.len();

        &self.buffer[buf..buf + self.size_x]
    }

    fn update_screen(&mut self, update: OutputUpdate) {
        let video = video();

        match update {
            OutputUpdate::Line(line, num) => {
                for l in line..line + num {
                    video.copy_txt_buffer(0, l, self.get_buffer_line(l));
                }
            }
            OutputUpdate::Viewport => {
                if self.viewport_y * self.size_x + self.size_x * self.size_y < self.buffer.len() {
                    video.copy_txt_buffer(
                        0,
                        0,
                        &self.buffer[self.viewport_y * self.size_x
                            ..self.viewport_y * self.size_x + self.size_x * self.size_y],
                    );
                } else {
                    let split = self.buffer.len() - self.viewport_y * self.size_x;
                    let split_lines = self.buffer_lines - self.viewport_y;

                    video.copy_txt_buffer(0, 0, &self.buffer[self.viewport_y * self.size_x..]);
                    video.copy_txt_buffer(
                        0,
                        split_lines,
                        &self.buffer[..self.size_x * self.size_y - split],
                    );
                }
            }
        }

        video.update_cursor(self.cursor_x, self.cursor_y);
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

        let target_viewport_y = (self.buffer_start_y + target_line) % self.buffer_lines;

        if target_viewport_y != self.viewport_y {
            self.viewport_y = target_viewport_y;

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

            self.viewport_y = (self.buffer_start_y + current_line) % self.buffer_lines;

            true
        } else {
            false
        }
    }

    fn _scroll_bottom(&mut self) -> bool {
        let max_line = self.max_line();

        let current_line = self.viewport_line();

        if current_line < max_line {
            self.viewport_y = (self.buffer_start_y + max_line) % self.buffer_lines;

            true
        } else {
            false
        }
    }

    fn _scroll_top(&mut self) -> bool {
        if self.viewport_line() > self.size_y {
            self.viewport_y = self.buffer_start_y;

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
        log!("{}", char as char);

        let mut update = OutputUpdate::Line(self.cursor_y, 1);

        if self._scroll_bottom() {
            update = OutputUpdate::Viewport;
        }

        self.store_char(char, &mut update);

        self.scroll(&mut update);

        self.update_screen(update);
    }

    pub fn write_str(&mut self, str: &str) {
        log!("{}", str);
        let mut update = OutputUpdate::Line(self.cursor_y, 1);

        if self._scroll_bottom() {
            update = OutputUpdate::Viewport;
        }

        for c in str.as_bytes().iter() {
            self.store_char(*c, &mut update);
        }
        self.scroll(&mut update);

        self.update_screen(update);
    }

    pub fn remove_last_n(&mut self, mut n: usize) {
        let blank = ScreenChar::new(b' ', ColorCode::new(Color::LightGreen, Color::Black));

        let mut update = OutputUpdate::Line(self.cursor_y, 1);

        if self._scroll_bottom() {
            update = OutputUpdate::Viewport;
        }

        while n > 0 && (self.cursor_x != 0 || self.cursor_y != 0) {
            let pos = self.cursor_buf_pos() - 1;

            self.buffer[pos] = blank;

            if self.cursor_x == 0 {
                self.cursor_x = self.size_x - 1;
                self.cursor_y -= 1;

                update.move_prev();
            } else {
                self.cursor_x -= 1;
            }
            n -= 1;
        }

        self.update_screen(update);
    }
}
