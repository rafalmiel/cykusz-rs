use crate::arch::output::video;
use crate::drivers::multiboot2::framebuffer_info::FramebufferInfo;
use crate::drivers::tty::palette;

pub mod fb;
pub mod vga;

pub fn init(fb_info: Option<&'static FramebufferInfo>) {
    palette::init_palette();
    if let Some(fb) = fb_info {
        if fb.typ() == 2 {
            vga::init();
        } else {
            fb::init(fb);
        }
    }
    let w = video();
    w.clear()
}
