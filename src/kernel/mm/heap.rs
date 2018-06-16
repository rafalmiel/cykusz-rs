use core::alloc::{Alloc, Layout, AllocErr, GlobalAlloc};
use core::ops::Deref;
use core::ptr::NonNull;

use linked_list_allocator::{Heap, align_up};

use kernel::mm::*;
use kernel::mm::PAGE_SIZE;
use kernel::mm::map;
use spin::Mutex;

use arch::mm::heap::{HEAP_START, HEAP_END, HEAP_SIZE};

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

    unsafe fn allocate(&self, heap: &mut Heap, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        heap.alloc(layout.clone()).or_else(|e| {
            match e {
                AllocErr{ .. } => {
                    let top = heap.top();
                    let req = align_up(layout.size(), 0x1000);

                    if top as usize + req as usize > HEAP_END.0 {
                        panic!("Out of mem!");
                    }

                    map_more_heap(top as *const u8, req);

                    heap.extend(req);

                    heap.alloc(layout)
                },
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

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(&mut self.0.lock(), layout).ok().map_or(0 as *mut u8, |alloc| alloc.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub fn allocate(layout: Layout) -> Option<*mut u8> {
    unsafe {
        Some(::HEAP.alloc(layout) as *mut u8)
    }
}
