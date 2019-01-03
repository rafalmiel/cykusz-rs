pub use arch::mm::{MappedAddr, PhysAddr, VirtAddr};
pub use arch::mm::PAGE_SIZE;
pub use arch::mm::phys::allocate;
pub use arch::mm::phys::deallocate;
pub use arch::mm::virt::map;
pub use arch::mm::virt::map_flags;
pub use arch::mm::virt::map_to;
pub use arch::mm::virt::unmap;

pub use self::frame::Frame;

mod frame;
pub mod virt;
pub mod heap;

pub fn init() {
    heap::init();

    println!("[ OK ] Heap Initialized");
}
