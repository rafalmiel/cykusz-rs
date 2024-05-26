pub use crate::arch::mm::{MappedAddr, PhysAddr, VirtAddr};
pub use crate::arch::mm::MAX_USER_ADDR;
pub use crate::arch::mm::MMAP_USER_ADDR;
pub use crate::arch::mm::PAGE_SIZE;
pub use crate::arch::mm::phys::allocate;
pub use crate::arch::mm::phys::allocate_order;
pub use crate::arch::mm::phys::deallocate;
pub use crate::arch::mm::phys::deallocate_order;
pub use crate::arch::mm::phys::free_mem;
pub use crate::arch::mm::phys::used_mem;
pub use crate::arch::mm::virt::get_flags;
pub use crate::arch::mm::virt::map;
pub use crate::arch::mm::virt::map_flags;
pub use crate::arch::mm::virt::map_to;
pub use crate::arch::mm::virt::map_to_flags;
pub use crate::arch::mm::virt::to_phys;
pub use crate::arch::mm::virt::unmap;
pub use crate::arch::mm::virt::update_flags;

pub use self::frame::Frame;

mod frame;
pub mod heap;
pub mod virt;

pub fn init() {
    heap::init();
}
