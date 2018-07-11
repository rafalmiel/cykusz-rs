mod frame;
pub mod virt;
pub mod heap;

pub use self::frame::Frame;
pub use crate::arch::mm::{PhysAddr, VirtAddr, MappedAddr};
pub use crate::arch::mm::PAGE_SIZE;

pub use crate::arch::mm::phys::allocate;
pub use crate::arch::mm::phys::deallocate;
pub use crate::arch::mm::virt::map_flags;
pub use crate::arch::mm::virt::map;
pub use crate::arch::mm::virt::unmap;
pub use crate::arch::mm::virt::map_to;

pub fn init() {
    heap::init();

    println!("[ OK ] Heap Initialized");
}
