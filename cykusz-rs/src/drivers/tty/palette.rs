use spin::Once;
use crate::drivers::tty::color::RGB;

pub const PALETTE_SIZE: usize = 256;

pub type Palette = [RGB; PALETTE_SIZE];

// 256 colors Ansi palette: https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
static DEFAULT_PALETTE: Once<Palette> = Once::new();

pub fn init_palette() {
    DEFAULT_PALETTE.call_once(|| {
        let mut palette = [RGB::new(0, 0, 0); PALETTE_SIZE];

        palette[0] = RGB::new(0,0,0);       // Black
        palette[1] = RGB::new(170,0,0);     // Red
        palette[2] = RGB::new(0,170,0);     // Green
        palette[3] = RGB::new(170,85,0);    // Yellow
        palette[4] = RGB::new(0,0,170);     // Blue
        palette[5] = RGB::new(170,0,170);   // Magenta
        palette[6] = RGB::new(0,170,170);   // Cyan
        palette[7] = RGB::new(170,170,170); // White
        palette[8] = RGB::new(85,85,85);    // Bright Black
        palette[9] = RGB::new(255,85,85);   // Bright Red
        palette[10] = RGB::new(85,255,85);  // Bright Green
        palette[11] = RGB::new(255,255,85); // Bright Yellow
        palette[12] = RGB::new(85,85,255);  // Bright Blue
        palette[13] = RGB::new(255,85,255); // Bright Magenta
        palette[14] = RGB::new(85,255,255); // Bright Cyan
        palette[15] = RGB::new(255,255,255);// Bright White

        for red in 0..6 {
            for green in 0..6 {
                for blue in 0..6 {
                    palette[16 + blue + 6 * green + 36 * red] = RGB::new(
                        red as u8 * 40 + if red != 0 { 55 } else { 0 },
                        green as u8 * 40 + if red != 0 { 55 } else { 0 },
                        blue as u8 * 40 + if red != 0 { 55 } else { 0 },
                    );
                }
            }
        }

        for gray in 0..24 {
            let val = gray as u8 * 10 + 8;
            palette[232 + gray] = RGB::new(val, val, val);
        }

        palette
    });
}

pub fn palette() -> &'static Palette {
    unsafe {
        DEFAULT_PALETTE.get_unchecked()
    }
}