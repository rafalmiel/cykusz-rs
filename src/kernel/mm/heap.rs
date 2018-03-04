use alloc::allocator::{Alloc, Layout, AllocErr};
use core::ops::Deref;

use linked_list_allocator::{Heap, align_up};

use kernel::mm::*;
use kernel::mm::PAGE_SIZE;
use kernel::mm::map;
use spin::Mutex;

pub const HEAP_START: VirtAddr = VirtAddr(0xfffff80000000000);
pub const HEAP_SIZE: usize = 1 * 4096; // 4KB / 1 pages // heap will grow when more memory is needed
pub const HEAP_END: VirtAddr = VirtAddr(HEAP_START.0 + (4096 * 4096) as usize); // 4MB

pub fn init()
{
    use HEAP;

    for addr in (HEAP_START..(HEAP_START + HEAP_SIZE)).step_by(PAGE_SIZE) {
        map(addr);
    }
    unsafe {
        HEAP.0.lock().init(HEAP_START.0, HEAP_SIZE);
    }
}

fn map_more_heap(from: *const u8, size: usize) {
    for addr in (VirtAddr(from as usize)..VirtAddr(from as usize) + size).step_by(PAGE_SIZE) {
        map(addr);
    }
}

pub struct LockedHeap(pub Mutex<Heap>);

impl LockedHeap {
    /// Creates an empty heap. All allocate calls will return `None`.
    pub const fn empty() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::empty()))
    }

    unsafe fn allocate(&self, heap: &mut Heap, layout: Layout) -> Result<*mut u8, AllocErr> {
        use self::AllocErr::{Exhausted, Unsupported};

        heap.alloc(layout.clone()).or_else(|e| {
            match e {
                Exhausted{ .. } => {
                    let top = heap.top();
                    let req = align_up(layout.size(), 0x1000);

                    if top as usize + req as usize > HEAP_END.0 {
                        panic!("Out of mem!");
                    }

                    map_more_heap(top as *const u8, req);

                    heap.extend(req);

                    heap.alloc(layout)
                },
                Unsupported{ details: s} => {
                    panic!("Out of mem! {}", s);
                }
            }
        })
    }
}

impl Deref for LockedHeap {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Mutex<Heap> {
        &self.0
    }
}

unsafe impl Alloc for LockedHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        self.allocate(&mut self.0.lock(), layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout)
    }
}

unsafe impl<'a> Alloc for &'a LockedHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        self.allocate(&mut self.0.lock(), layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout)
    }
}
