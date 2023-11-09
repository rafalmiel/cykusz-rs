pub const TCGETS: usize = 0x5401;
pub const TCSETS: usize = 0x5402;
pub const TCSETSW: usize = 0x5403;
pub const TCSETSF: usize = 0x5404;

pub const TIOCSCTTY: usize = 0x540E;
pub const TIOCNOTTY: usize = 0x5422;
pub const TIOCGPGRP: usize = 0x540f;
pub const TIOCSPGRP: usize = 0x5410;

pub const TIOCGWINSZ: usize = 0x5413;
pub const TIOCSWINSZ: usize = 0x5414;

#[repr(C)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

// indices for the c_cc array in struct termios
pub const NCCS: u32 = 32;
pub const VINTR: u32 = 0;
pub const VQUIT: u32 = 1;
pub const VERASE: u32 = 2;
pub const VKILL: u32 = 3;
pub const VEOF: u32 = 4;
pub const VTIME: u32 = 5;
pub const VMIN: u32 = 6;
pub const VSWTC: u32 = 7;
pub const VSTART: u32 = 8;
pub const VSTOP: u32 = 9;
pub const VSUSP: u32 = 10;
pub const VEOL: u32 = 11;
pub const VREPRINT: u32 = 12;
pub const VDISCARD: u32 = 13;
pub const VWERASE: u32 = 14;
pub const VLNEXT: u32 = 15;
pub const VEOL2: u32 = 16;

// bitwise flags for c_iflag in struct termios
pub const BRKINT: u32 = 0000002;
pub const ICRNL: u32 = 0000400;
pub const IGNBRK: u32 = 0000001;
pub const IGNCR: u32 = 0000200;
pub const IGNPAR: u32 = 0000004;
pub const INLCR: u32 = 0000100;
pub const INPCK: u32 = 0000020;
pub const ISTRIP: u32 = 0000040;
pub const IXANY: u32 = 0004000;
pub const IXOFF: u32 = 0010000;
pub const IXON: u32 = 0002000;
pub const PARMRK: u32 = 0000010;

// bitwise flags for c_oflag in struct termios
pub const OPOST: u32 = 0000001;
pub const ONLCR: u32 = 0000004;
pub const OCRNL: u32 = 0000010;
pub const ONOCR: u32 = 0000020;
pub const ONLRET: u32 = 0000040;
pub const OFDEL: u32 = 0000200;
pub const OFILL: u32 = 0000100;

pub const NLDLY: u32 = 0000400;
pub const NL0: u32 = 0000000;
pub const NL1: u32 = 0000400;

pub const CRDLY: u32 = 0003000;
pub const CR0: u32 = 0000000;
pub const CR1: u32 = 0001000;
pub const CR2: u32 = 0002000;
pub const CR3: u32 = 0003000;

pub const TABDLY: u32 = 0014000;
pub const TAB0: u32 = 0000000;
pub const TAB1: u32 = 0004000;
pub const TAB2: u32 = 0010000;
pub const TAB3: u32 = 0014000;

pub const XTABS: u32 = 0014000;
pub const BSDLY: u32 = 0020000;
pub const BS0: u32 = 0000000;
pub const BS1: u32 = 0020000;

pub const VTDLY: u32 = 0040000;
pub const VT0: u32 = 0000000;
pub const VT1: u32 = 0040000;

pub const FFDLY: u32 = 0100000;
pub const FF0: u32 = 0000000;
pub const FF1: u32 = 0100000;

// bitwise constants for c_cflag in struct termios
pub const CSIZE: u32 = 0000060;
pub const CS5: u32 = 0000000;
pub const CS6: u32 = 0000020;
pub const CS7: u32 = 0000040;
pub const CS8: u32 = 0000060;

pub const CSTOPB: u32 = 0000100;
pub const CREAD: u32 = 0000200;
pub const PARENB: u32 = 0000400;
pub const PARODD: u32 = 0001000;
pub const HUPCL: u32 = 0002000;
pub const CLOCAL: u32 = 0004000;

// bitwise constants for c_lflag in struct termios
pub const ECHO: u32 = 0000010;
pub const ECHOE: u32 = 0000020;
pub const ECHOK: u32 = 0000040;
pub const ECHONL: u32 = 0000100;
pub const ICANON: u32 = 0000002;
pub const IEXTEN: u32 = 0100000;
pub const ISIG: u32 = 0000001;
pub const NOFLSH: u32 = 0000200;
pub const TOSTOP: u32 = 0000400;
pub const ECHOPRT: u32 = 0002000;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub ibaud: u32,
    pub obaud: u32,
}

impl Termios {
    pub const fn default() -> Termios {
        Termios {
            c_iflag: IXOFF | IXON | ICRNL,
            c_oflag: OPOST | ONLCR,
            c_cflag: CREAD,
            c_lflag: IEXTEN | ECHOK | ECHOE | ECHO | ICANON | ISIG,
            c_line: 0,
            c_cc: [0; 32],
            ibaud: 0,
            obaud: 0,
        }
    }

    pub fn has_iflag(&self, iflag: u32) -> bool {
        self.c_iflag & iflag == iflag
    }
    pub fn has_oflag(&self, oflag: u32) -> bool {
        self.c_oflag & oflag == oflag
    }
    pub fn has_cflag(&self, cflag: u32) -> bool {
        self.c_cflag & cflag == cflag
    }
    pub fn has_lflag(&self, lflag: u32) -> bool {
        self.c_lflag & lflag == lflag
    }
}
