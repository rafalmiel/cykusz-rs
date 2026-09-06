use super::buddy::BuddyAlloc;
use super::iter::RangeMemIterator;
use super::slab::SlabAlloc;
use crate::arch::mm::VirtAddr;
use crate::arch::mm::phys::{MemZone, buddy};
use crate::drivers::multiboot2;
use crate::drivers::multiboot2::memory::MemoryIter;
use crate::kernel::mm::{Frame, PAGE_SIZE};
use crate::kernel::mm::{MappedAddr, PhysAddr};
use crate::kernel::sync::{LockApi, Spin};
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

pub static NUM_PAGES: AtomicU64 = AtomicU64::new(0);

static BUDDY: Spin<BuddyAlloc> = Spin::new(BuddyAlloc::new());

static SLAB: spin::Once<Spin<SlabAlloc>> = spin::Once::new();

pub fn allocate_slab(size: usize) -> Option<VirtAddr> {
    allocate_slab_zone(size, MemZone::ZoneNormal)
}

pub fn allocate_slab_zone(size: usize, zone: MemZone) -> Option<VirtAddr> {
    let mut slab = unsafe { SLAB.get_unchecked().lock_irq() };

    slab.allocate_zone(size, zone)
}

pub fn free_slab(addr: VirtAddr) {
    let mut slab = unsafe { SLAB.get_unchecked().lock_irq() };

    slab.free(addr);
}

pub fn allocate() -> Option<Frame> {
    allocate_order(0)
}

pub fn allocate_zone(zone: MemZone) -> Option<Frame> {
    allocate_order_zone(0, zone)
}

pub fn allocate_order(order: usize) -> Option<Frame> {
    allocate_order_zone(order, MemZone::ZoneNormal)
}

pub fn allocate_order_zone(order: usize, zone: MemZone) -> Option<Frame> {
    let mut bdy = BUDDY.lock_irq();

    if let Some(addr) = bdy.alloc_zone(order, zone) {
        Some(Frame::new(addr))
    } else {
        None
    }
}

pub fn order_for_size(size: usize) -> Option<usize> {
    let max_order = buddy::BUDDY_COUNT;

    let mut order = 0;
    let mut current_size = PAGE_SIZE;

    while order < max_order && size > current_size {
        current_size <<= 1;
        order += 1;
    }

    if order >= max_order {
        None
    } else {
        Some(order)
    }
}

pub fn deallocate(frame: &Frame) {
    deallocate_order(frame, 0);
}

pub fn deallocate_order(frame: &Frame, order: usize) {
    let mut bdy = BUDDY.lock_irq();

    bdy.dealloc(frame.address(), order);

    let size = buddy::BSIZE[order];

    for p in (frame.address()..frame.address() + size).step_by(PAGE_SIZE) {
        p.to_phys_page().unwrap().mark_unused();
    }
}

pub fn used_mem() -> usize {
    let bdy = BUDDY.lock_irq();

    bdy.used_mem()
}

pub fn free_mem() -> usize {
    let bdy = BUDDY.lock_irq();

    bdy.free_mem()
}

fn init_slab() {
    SLAB.call_once(|| Spin::new(SlabAlloc::new()));
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

    //logln!("init mem {} {}", mem_start, mem_end);
    bdy.init(mem_start, mem_end);

    for &(s, l) in ranges[..len].iter() {
        //logln!("add range {} {}", s, s + l);
        bdy.add_range(s, s + l);
    }

    NUM_PAGES.store((mem_end.0 / PAGE_SIZE) as u64, Ordering::SeqCst);

    init_slab();
}
