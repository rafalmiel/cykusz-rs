pub const TCGETS: usize = 0x5401;
pub const TCSETS: usize = 0x5402;

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

// bitwise flags for c_iflag in struct termios
pub const BRKINT: u32 = 0x0001;
pub const ICRNL: u32 = 0x000;
pub const IGNBRK: u32 = 0x000;
pub const IGNCR: u32 = 0x000;
pub const IGNPAR: u32 = 0x001;
pub const INLCR: u32 = 0x002;
pub const INPCK: u32 = 0x004;
pub const ISTRIP: u32 = 0x008;
pub const IXANY: u32 = 0x010;
pub const IXOFF: u32 = 0x020;
pub const IXON: u32 = 0x040;
pub const PARMRK: u32 = 0x080;

// bitwise flags for c_oflag in struct termios
pub const OPOST: u32 = 0x0001;
pub const ONLCR: u32 = 0x0002;
pub const OCRNL: u32 = 0x0004;
pub const ONOCR: u32 = 0x0008;
pub const ONLRET: u32 = 0x0010;
pub const OFDEL: u32 = 0x0020;
pub const OFILL: u32 = 0x0040;

pub const NLDLY: u32 = 0x0080;
pub const NL0: u32 = 0x0000;
pub const NL1: u32 = 0x0080;

pub const CRDLY: u32 = 0x0300;
pub const CR0: u32 = 0x0000;
pub const CR1: u32 = 0x0100;
pub const CR2: u32 = 0x0200;
pub const CR3: u32 = 0x0300;

pub const TABDLY: u32 = 0x0C00;
pub const TAB0: u32 = 0x0000;
pub const TAB1: u32 = 0x0400;
pub const TAB2: u32 = 0x0800;
pub const TAB3: u32 = 0x0C00;

pub const BSDLY: u32 = 0x1000;
pub const BS0: u32 = 0x0000;
pub const BS1: u32 = 0x1000;

pub const VTDLY: u32 = 0x2000;
pub const VT0: u32 = 0x0000;
pub const VT1: u32 = 0x2000;

pub const FFDLY: u32 = 0x4000;
pub const FF0: u32 = 0x0000;
pub const FF1: u32 = 0x4000;

// baud rate constants for speed_t
pub const B0: u32 = 0;
pub const B50: u32 = 1;
pub const B75: u32 = 2;
pub const B110: u32 = 3;
pub const B134: u32 = 4;
pub const B150: u32 = 5;
pub const B200: u32 = 6;
pub const B300: u32 = 7;
pub const B600: u32 = 8;
pub const B1200: u32 = 9;
pub const B1800: u32 = 10;
pub const B2400: u32 = 11;
pub const B4800: u32 = 12;
pub const B9600: u32 = 13;
pub const B19200: u32 = 14;
pub const B38400: u32 = 15;
pub const B57600: u32 = 16;
pub const B115200: u32 = 17;
pub const B230400: u32 = 18;

// bitwise constants for c_cflag in struct termios
pub const CSIZE: u32 = 0x0003;
pub const CS5: u32 = 0x0000;
pub const CS6: u32 = 0x0001;
pub const CS7: u32 = 0x0002;
pub const CS8: u32 = 0x0003;

pub const CSTOPB: u32 = 0x0004;
pub const CREAD: u32 = 0x0008;
pub const PARENB: u32 = 0x0010;
pub const PARODD: u32 = 0x0020;
pub const HUPCL: u32 = 0x0040;
pub const CLOCAL: u32 = 0x0080;

// bitwise constants for c_lflag in struct termios
pub const ECHO: u32 = 0x0001;
pub const ECHOE: u32 = 0x0002;
pub const ECHOK: u32 = 0x0004;
pub const ECHONL: u32 = 0x0008;
pub const ICANON: u32 = 0x0010;
pub const IEXTEN: u32 = 0x0020;
pub const ISIG: u32 = 0x0040;
pub const NOFLSH: u32 = 0x0080;
pub const TOSTOP: u32 = 0x0100;
pub const ECHOPRT: u32 = 0x0200;

pub const ECHOCTL: u32 = 0o0001000;
pub const FLUSHO: u32 = 0o0010000;
pub const IMAXBEL: u32 = 0o0020000;
pub const ECHOKE: u32 = 0o0040000;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_cc: [u32; 11],
    pub ibaud: u32,
    pub obaud: u32,
}

impl Termios {
    pub const fn default() -> Termios {
        Termios {
            c_iflag: IXOFF | IXON | ICRNL,
            c_oflag: OPOST | ONLCR,
            c_cflag: CREAD,
            c_lflag: IEXTEN | ECHOKE | ECHOK | ECHOE | ECHO | ICANON | ISIG,
            c_cc: [0; 11],
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
