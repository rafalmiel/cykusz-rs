pub const GFBINFO: usize = 0x3415;

pub struct FbInfo {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u64,
}