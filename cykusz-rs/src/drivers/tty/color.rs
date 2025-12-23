use crate::drivers::tty::palette::palette;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum ColorCode {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    LightBlack = 8,
    LightRed = 9,
    LightGreen = 10,
    LightYellow = 11,
    LightBlue = 12,
    LightMagenta = 13,
    LightCyan = 14,
    LightWhite = 15,
}

impl From<usize> for ColorCode {
    fn from(value: usize) -> Self {
        (value as u8).into()
    }
}

impl From<u8> for ColorCode {
    fn from(value: u8) -> Self {
        if value > 15 {
            panic!("Invalid usize to ColorCode conversion, value: {}", value)
        }

        unsafe {
            core::mem::transmute(value as u8)
        }
    }
}

fn find_closest(rgb: &RGB, palette: &[RGB]) -> usize {
    let mut closest: Option<usize> = None;
    for (idx, chunk) in palette.iter().enumerate() {
        let rdist = (chunk.red() as usize - rgb.red() as usize) << 1;
        let gdist = (chunk.green() as usize - rgb.green() as usize) << 1;
        let bdist = (chunk.blue() as usize - rgb.blue() as usize) << 1;
        let dist = rdist + gdist + bdist;
        if closest.is_none() || dist < closest.unwrap() {
            closest = Some(idx)
        }
    }

    closest.unwrap()
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Ansi16 {
    c: ColorCode
}

impl From<Ansi256> for Ansi16 {
    fn from(value: Ansi256) -> Self {
        Into::<RGB>::into(value).into()
    }
}

impl From<RGB> for Ansi16 {
    fn from(value: RGB) -> Self {
        Ansi16 {
            c: find_closest(&value, &palette()[0..16]).into()
        }
    }
}

impl Ansi16 {
    pub const fn new(c: ColorCode) -> Ansi16 {
        Ansi16 {
            c
        }
    }

    pub const fn color(&self) -> ColorCode {
        self.c
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Ansi256 {
    c: u8,
}

impl From<Ansi16> for Ansi256 {
    fn from(value: Ansi16) -> Self {
        Ansi256 {
            c: value.color() as u8
        }
    }
}

impl From<RGB> for Ansi256 {
    fn from(value: RGB) -> Self {
        Ansi256 {
            c: find_closest(&value, palette()) as u8
        }
    }
}

impl Ansi256 {
    pub fn new(c: u8) -> Self {
        Ansi256 { c }
    }

    pub fn color(&self) -> u8 {
        self.c
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct RGB {
    colors: [u8; 3],
}

impl From<Ansi16> for RGB {
    fn from(value: Ansi16) -> Self {
        let idx = value.color() as usize;
        let val = palette()[idx as u8 as usize];

        val
    }
}

impl From<Ansi256> for RGB {
    fn from(value: Ansi256) -> Self {
        let idx = value.color() as usize;
        let val = palette()[idx as u8 as usize];

        val
    }
}

impl RGB {
    pub const fn new(r: u8, g: u8, b: u8) -> RGB {
        RGB {
            colors: [r, g, b]
        }
    }

    pub const fn from_slice(colors: &[u8]) -> RGB {
        RGB {
            colors: [colors[0], colors[1], colors[2]]
        }
    }

    pub const fn red(&self) -> u8 {
        self.colors[0]
    }

    pub const fn green(&self) -> u8 {
        self.colors[1]
    }

    pub const fn blue(&self) -> u8 {
        self.colors[2]
    }

    pub fn set_red(&mut self, red: u8) {
        self.colors[0] = red;
    }

    pub fn set_green(&mut self, green: u8) {
        self.colors[1] = green;
    }

    pub fn set_blue(&mut self, blue: u8) {
        self.colors[2] = blue;
    }

    pub fn dim(&self) -> RGB {
        *self
    }

    pub fn brighten(&self) -> RGB {
        *self
    }
}