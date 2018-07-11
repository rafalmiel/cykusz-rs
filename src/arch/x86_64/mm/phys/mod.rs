mod alloc;
mod iter;

pub use self::alloc::allocate;
pub use self::alloc::deallocate;

use crate::drivers::multiboot2;

pub fn init(mboot_info: &multiboot2::Info) {
    alloc::init(mboot_info);
}
