use core::alloc::{Alloc, AllocErr, GlobalAlloc, Layout};
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use linked_list_allocator::{align_up, Heap};

use arch::mm::heap::{HEAP_END, HEAP_SIZE, HEAP_START};
use kernel::mm::*;
use kernel::mm::map;
use kernel::mm::PAGE_SIZE;
use kernel::sync::Mutex;

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

pub static ALLOCED_MEM: AtomicUsize = AtomicUsize::new(0);

impl LockedHeap {
    /// Creates an empty heap. All allocate calls will return `None`.
    pub const fn empty() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::empty()))
    }

    unsafe fn allocate(&self, heap: &mut Heap, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        ALLOCED_MEM.fetch_add(layout.size(), Ordering::SeqCst);
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
        ALLOCED_MEM.fetch_sub(layout.size(), Ordering::SeqCst);
        self.0.lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub fn allocate_layout(layout: Layout) -> Option<*mut u8> {
    unsafe {
        Some(::HEAP.alloc(layout) as *mut u8)
    }
}

pub fn deallocate_layout(ptr: *mut u8, layout: Layout) {
    unsafe {
        ::HEAP.dealloc(ptr, layout)
    }
}

pub fn allocate(size: usize) -> Option<*mut u8> {
    unsafe {
        Some(::HEAP.alloc(::core::alloc::Layout::from_size_align_unchecked(size, 8)) as *mut u8)
    }
}

pub fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        ::HEAP.dealloc(ptr, ::core::alloc::Layout::from_size_align_unchecked(size, 8))
    }
}

pub fn allocate_align(size: usize, align: usize) -> Option<*mut u8> {
    unsafe {
        Some(::HEAP.alloc(::core::alloc::Layout::from_size_align_unchecked(size, align)) as *mut u8)
    }
}

pub fn deallocate_align(ptr: *mut u8, size: usize, align: usize) {
    unsafe {
        ::HEAP.dealloc(ptr, ::core::alloc::Layout::from_size_align_unchecked(size, align))
    }
}
