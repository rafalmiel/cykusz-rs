pub const TIOCSCTTY: usize = 1;
pub const TIOCNOTTY: usize = 2;
pub const TIOCSPGRP: usize = 3;

pub const TIOCGWINSZ: usize = 4;
pub const TIOCSWINSZ: usize = 5;

#[repr(C)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}
