use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use crate::drivers::multiboot2;
use crate::drivers::multiboot2::memory::MemoryIter;
use crate::kernel::mm::{Frame, PAGE_SIZE};
use crate::kernel::mm::{MappedAddr, PhysAddr};
use crate::kernel::sync::Spin;

use super::buddy::BuddyAlloc;
use super::iter::RangeMemIterator;

pub static NUM_PAGES: AtomicU64 = AtomicU64::new(0);

static BUDDY: Spin<BuddyAlloc> = Spin::new(BuddyAlloc::new());

pub fn allocate() -> Option<Frame> {
    let mut bdy = BUDDY.lock();

    if let Some(addr) = bdy.alloc(0) {
        Some(Frame::new(addr))
    } else {
        None
    }
}

pub fn deallocate(frame: &Frame) {
    let mut bdy = BUDDY.lock();

    bdy.dealloc(frame.address(), 0);
}

pub fn init(mboot_info: &multiboot2::Info) {
    let mem = mboot_info
        .memory_map_tag()
        .expect("Memory map tag not found");
    let mm_iter: MemoryIter = mem.entries();
    let kern_start: PhysAddr = mboot_info.kernel_start_addr();
    let kern_end: PhysAddr = mboot_info.kernel_end_addr();
    let mboot_start: PhysAddr = MappedAddr(mboot_info as *const _ as usize).to_phys();
    let mboot_end: PhysAddr = mboot_start + mboot_info.size as usize;
    let modules_start: PhysAddr = mboot_info.modules_start_addr().unwrap_or_default();
    let modules_end: PhysAddr = mboot_info.modules_end_addr().unwrap_or_default();

    let riter = RangeMemIterator::new(
        mm_iter,
        kern_start,
        kern_end,
        mboot_start,
        mboot_end,
        modules_start,
        modules_end,
    );

    let mut ranges: [(PhysAddr, usize); 16] = [(PhysAddr(0), 0); 16];
    let mut len = 0;

    for (s, size) in riter {
        ranges[len] = (s, size);
        len += 1;
    }

    assert!(len > 0, "No mem detected?");

    let mem_start = ranges[0].0;
    let mem_end = ranges[len - 1].0 + ranges[len - 1].1;

    let mut bdy = BUDDY.lock();

    bdy.init(mem_start, mem_end);

    for e in 0..len {
        bdy.add_range(ranges[e].0, ranges[e].0 + ranges[e].1);
    }

    NUM_PAGES.store((mem_end.0 / PAGE_SIZE) as u64, Ordering::SeqCst);
}
