use crate::drivers::multiboot2::Tag;

#[repr(packed)]
pub struct CommandLine {
    pub tag: Tag,
    cmdline_byte: u8,
}

impl CommandLine {
    pub fn command_line(&self) -> &str {
        use ::core::{mem, slice, str};
        let strlen = self.tag.size as usize - mem::size_of::<CommandLine>();
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(&self.cmdline_byte as *const u8, strlen))
        }
    }
}
