mod frame;
pub mod virt;

pub use self::frame::Frame;
pub use arch::mm::{PhysAddr, VirtAddr, MappedAddr};
pub use arch::mm::PAGE_SIZE;

pub use arch::mm::phys::allocate;
pub use arch::mm::phys::deallocate;
pub use arch::mm::virt::map_flags;
pub use arch::mm::virt::map;
pub use arch::mm::virt::unmap;
pub use arch::mm::virt::map_to;
