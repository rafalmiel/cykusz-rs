use crate::drivers::multiboot2::Tag;

#[repr(C)]
#[derive(Debug)]
pub struct FramebufferInfo {
    pub tag: Tag,
    addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    typ: u8,
    _reserved: u16,
    color_info: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct FramebufferType {
    red_field_pos: u8,
    red_mask_size: u8,
    green_field_pos: u8,
    green_mask_size: u8,
    blue_field_pos: u8,
    blue_mask_size: u8,
}

impl FramebufferInfo {
    pub fn addr(&self) -> u64 {
        self.addr
    }
    pub fn pitch(&self) -> u32 {
        self.pitch
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn bpp(&self) -> u8 {
        self.bpp
    }
    pub fn typ(&self) -> u8 {
        self.typ
    }

    pub fn framebuffer_type(&self) -> &'static FramebufferType {
        unsafe {
            &*(&self.color_info as *const u8 as *const FramebufferType)
        }
    }
}