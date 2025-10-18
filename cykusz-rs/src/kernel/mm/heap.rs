use core::alloc::{GlobalAlloc, Layout};
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use linked_list_allocator::Heap;
use spin::Once;

use crate::arch::mm::heap::{HEAP_END, HEAP_SIZE, HEAP_START};
use crate::kernel::mm::*;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::types::Align;

pub fn init() {
    for addr in (HEAP_START..(HEAP_START + HEAP_SIZE)).step_by(PAGE_SIZE) {
        map(addr);
    }
    unsafe {
        let heap = heap();
        heap.0.lock_irq().init(HEAP_START.0 as *mut u8, HEAP_SIZE);
    }
    LEAK_CATCHER.call_once(|| LeakCatcher::new());
}

fn map_more_heap(from: *const u8, size: usize) {
    for addr in (VirtAddr(from as usize)..VirtAddr(from as usize) + size).step_by(PAGE_SIZE) {
        map(addr);
    }
}

pub struct LockedHeap(pub Spin<Heap>);

pub static ALLOCED_MEM: AtomicUsize = AtomicUsize::new(0);

static LEAK_CATCHER: Once<LeakCatcher> = Once::new();

pub fn leak_catcher() -> &'static LeakCatcher {
    LEAK_CATCHER.get().unwrap()
}

pub struct LeakCatcher {
    enabled: AtomicBool,
    allocs: Spin<hashbrown::HashMap<usize, Layout>>,
}

impl LeakCatcher {
    #[allow(unused)]
    fn new() -> LeakCatcher {
        LeakCatcher {
            enabled: AtomicBool::new(false),
            allocs: Spin::new(hashbrown::HashMap::new()),
        }
    }

    pub fn track_alloc(&self, ptr: usize, layout: Layout) {
        let enabled = self.is_enabled();

        if !enabled {
            return;
        }

        self.disable();

        if let Some(p) = self.allocs.lock().insert(ptr, layout) {
            dbgln!(
                leakcatcher,
                "replacing 0x{:x} {} with {}",
                ptr,
                layout.size(),
                p.size()
            );
        }

        if enabled {
            self.enable();
        }
    }
    pub fn track_dealloc(&self, ptr: usize) {
        let enabled = self.is_enabled();

        if !enabled {
            return;
        }

        self.disable();

        self.allocs.lock().remove(&ptr);

        if enabled {
            self.enable();
        }
    }

    pub fn report(&self) {
        let enabled = self.is_enabled();

        self.disable();

        let locks = self.allocs.lock();

        for p in locks.iter() {
            dbgln!(
                leakcatcher,
                "unallocated ptr: 0x{:x} size: {}",
                p.0,
                p.1.size()
            );
        }

        //locks.clear();

        if enabled {
            self.enable();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
}

pub fn heap_mem() -> usize {
    let heap = heap();
    let used = heap.lock_irq().used();

    used
}

impl LockedHeap {
    /// Creates an empty heap. All allocate calls will return `None`.
    pub const fn empty() -> LockedHeap {
        LockedHeap(Spin::new(Heap::empty()))
    }

    unsafe fn allocate(&self, heap: &mut Heap, layout: Layout) -> Result<NonNull<u8>, ()> { unsafe {
        ALLOCED_MEM.fetch_add(layout.size(), Ordering::SeqCst);
        heap.allocate_first_fit(layout.clone()).or_else(|_| {
            let _ = &heap;

            let top = heap.top();
            let req = layout.size().align_up(0x1000);

            if top as usize + req as usize > HEAP_END.0 {
                panic!("Out of mem!");
            }

            map_more_heap(top as *const u8, req);

            heap.extend(req);

            heap.allocate_first_fit(layout)
        })
    }}
}

impl Deref for LockedHeap {
    type Target = Spin<Heap>;

    fn deref(&self) -> &Spin<Heap> {
        &self.0
    }
}

pub static HEAP_DEBUG: AtomicBool = AtomicBool::new(false);

pub fn enable_heap_debug() {
    HEAP_DEBUG.store(true, Ordering::SeqCst);
}

pub fn disable_heap_debug() {
    HEAP_DEBUG.store(false, Ordering::SeqCst);
}

pub struct HeapDebug {}

impl HeapDebug {
    pub fn new() -> HeapDebug {
        HEAP_DEBUG.store(true, Ordering::SeqCst);

        HeapDebug {}
    }
}

impl Drop for HeapDebug {
    fn drop(&mut self) {
        HEAP_DEBUG.store(false, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { unsafe {
        //let _sw = if crate::kernel::smp::is_smp_initialised() {
        //    Some(crate::kernel::utils::stopwatch::StopWatch::new("alloc"))
        //} else {
        //    None
        //};
        let ptr = self
            .allocate(&mut self.0.lock_irq(), layout)
            .ok()
            .map_or(0 as *mut u8, |alloc| alloc.as_ptr());

        leak_catcher().track_alloc(ptr as usize, layout);
        if HEAP_DEBUG.load(Ordering::SeqCst) {
            println!("Alloc {:p} {}", ptr, layout.size());
        };

        ptr
    }}

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { unsafe {
        //let _sw = if crate::kernel::smp::is_smp_initialised() {
        //    Some(crate::kernel::utils::stopwatch::StopWatch::new("dealloc"))
        //} else {
        //    None
        //};
        leak_catcher().track_dealloc(ptr as usize);
        if HEAP_DEBUG.load(Ordering::SeqCst) {
            println!("Dealloc {:p} {}", ptr, layout.size());
        };
        ALLOCED_MEM.fetch_sub(layout.size(), Ordering::SeqCst);
        self.0
            .lock_irq()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }}
}

fn heap() -> &'static LockedHeap {
    unsafe { (&raw const crate::HEAP).as_ref_unchecked() }
}

pub fn allocate_layout(layout: Layout) -> Option<*mut u8> {
    unsafe { Some(heap().alloc(layout) as *mut u8) }
}

pub fn deallocate_layout(ptr: *mut u8, layout: Layout) {
    unsafe { heap().dealloc(ptr, layout) }
}

pub fn allocate(size: usize) -> Option<*mut u8> {
    unsafe {
        Some(heap().alloc(::core::alloc::Layout::from_size_align_unchecked(size, 8)) as *mut u8)
    }
}

pub fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        heap().dealloc(
            ptr,
            ::core::alloc::Layout::from_size_align_unchecked(size, 8),
        )
    }
}

pub fn allocate_align(size: usize, align: usize) -> Option<*mut u8> {
    unsafe {
        Some(
            heap().alloc(::core::alloc::Layout::from_size_align_unchecked(
                size, align,
            )) as *mut u8,
        )
    }
}

pub fn deallocate_align(ptr: *mut u8, size: usize, align: usize) {
    unsafe {
        heap().dealloc(
            ptr,
            ::core::alloc::Layout::from_size_align_unchecked(size, align),
        )
    }
}
