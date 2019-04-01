pub use crate::arch::mm::{MappedAddr, PhysAddr, VirtAddr};
pub use crate::arch::mm::PAGE_SIZE;
pub use crate::arch::mm::phys::allocate;
pub use crate::arch::mm::phys::deallocate;
pub use crate::arch::mm::virt::map;
pub use crate::arch::mm::virt::map_flags;
pub use crate::arch::mm::virt::map_to;
pub use crate::arch::mm::virt::unmap;

pub use self::frame::Frame;

mod frame;
pub mod virt;
pub mod heap;

pub fn init() {
    heap::init();

    println!("[ OK ] Heap Initialized");
}
