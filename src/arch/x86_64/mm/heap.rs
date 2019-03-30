use kernel::mm::VirtAddr;

pub const HEAP_START: VirtAddr = VirtAddr(0xfffff80000000000);
pub const HEAP_SIZE: usize = 1 * 4096; // 4KB / 1 pages // heap will grow when more memory is needed
pub const HEAP_END: VirtAddr = VirtAddr(HEAP_START.0 + (4096*4096) as usize); // 4MB

