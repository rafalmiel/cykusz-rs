use crate::drivers::multiboot2::tags::Tag;

#[repr(packed)]
pub struct Modules {
    pub tag: Tag,
    pub mod_start: u32,
    pub mod_end: u32,
    name_byte: u8,
}

impl Modules {
    pub fn name(&self) -> &str {
        use ::core::{mem, slice, str};
        let strlen = self.tag.size as usize - mem::size_of::<Modules>();
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(&self.name_byte as *const u8, strlen))
        }
    }
}
