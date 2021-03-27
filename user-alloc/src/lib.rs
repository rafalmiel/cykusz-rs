#![no_std]

mod types;

extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;

use syscall_defs::{MMapFlags, MMapProt};
use types::Align;

const HEAP_START: usize = 0x1000_0000;

struct Heap {
    heap_start: usize,
    heap_end: usize,
    heap: linked_list_allocator::Heap,
}

impl Heap {
    const fn empty() -> Heap {
        Heap {
            heap_start: HEAP_START,
            heap_end: HEAP_START,
            heap: linked_list_allocator::Heap::empty(),
        }
    }

    fn init(&mut self) {
        if self.heap.size() == 0 {
            if let Err(e) = syscall::mmap(
                Some(self.heap_start),
                4096,
                MMapProt::PROT_WRITE | MMapProt::PROT_READ,
                MMapFlags::MAP_PRIVATE | MMapFlags::MAP_ANONYOMUS | MMapFlags::MAP_FIXED,
                None,
                0,
            ) {
                panic!("mmap failed {:?}", e);
            }
            unsafe {
                self.heap.init(self.heap_start, 4096);
            }
            self.heap_end = self.heap_start + 4096;
        }
    }

    fn print_debug(&self) {
        println!(
            "heap {:p} {:#x} {:#x} top: {:#x}",
            self as *const _,
            self.heap_start,
            self.heap_end,
            self.heap.top()
        );
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.heap.allocate_first_fit(layout) {
            Ok(a) => a.as_ptr(),
            Err(_) => {
                self.extend_by(layout.size().align_up(4096));

                self.alloc(layout)
            }
        }
    }

    fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        unsafe { self.heap.deallocate(NonNull::new(ptr).unwrap(), layout) }
    }

    fn extend_by(&mut self, size: usize) {
        let size = size.align_up(4096);

        if let Ok(_) = syscall::mmap(
            Some(self.heap_end),
            size,
            MMapProt::PROT_WRITE | MMapProt::PROT_READ,
            MMapFlags::MAP_PRIVATE | MMapFlags::MAP_ANONYOMUS | MMapFlags::MAP_FIXED,
            None,
            0,
        ) {
            self.heap_end += size;

            unsafe { self.heap.extend(size) }
        } else {
            panic!("out of mem");
        }
    }
}

struct LockedHeap(spin::Mutex<Heap>);

impl LockedHeap {
    fn init(&self) {
        self.0.lock().init()
    }

    fn print_debug(&self) {
        self.0.lock().print_debug();
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout)
    }
}

#[global_allocator]
static HEAP: LockedHeap = LockedHeap(spin::Mutex::new(Heap::empty()));

pub fn init() {
    HEAP.init();
}

pub fn print_debug() {
    HEAP.print_debug();
}
