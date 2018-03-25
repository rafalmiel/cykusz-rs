use super::iter;

use spin::Mutex;
use kernel::mm::{PhysAddr, MappedAddr};
use kernel::mm::Frame;
use drivers::multiboot2;
use drivers::multiboot2::memory::MemoryIter;
use self::iter::PhysMemIterator;

const LIST_ADDR_INVALID: PhysAddr = PhysAddr(0xFFFF_FFFF_FFFF_FFFF);

fn is_list_addr_valid(addr: PhysAddr) -> bool {
    addr != LIST_ADDR_INVALID
}

struct PhysAllocatorList {
    head:           PhysAddr,
}

static PHYS_LIST: Mutex<PhysAllocatorList> = Mutex::new(
    PhysAllocatorList {
        head: LIST_ADDR_INVALID,
    }
);

pub fn allocate() -> Option<Frame> {
    let mut list = PHYS_LIST.lock();

    if is_list_addr_valid(list.head) {
        let ret = list.head;

        list.head = unsafe {
            list.head.to_mapped().read::<PhysAddr>()
        };

        let f = Frame::new(ret);

        return Some(f);
    }

    None
}

pub fn deallocate(frame: &Frame) {
    let mut list = PHYS_LIST.lock();

    unsafe {
        frame.address_mapped().store(list.head);
    }

    list.head = frame.address();
}

pub fn init(mboot_info: &multiboot2::Info) {

    let mem = mboot_info.memory_map_tag().expect("Memory map tag not found");
    let mm_iter:        MemoryIter = mem.entries();
    let kern_start:     PhysAddr = mboot_info.kernel_start_addr();
    let kern_end:       PhysAddr = mboot_info.kernel_end_addr();
    let mboot_start:    PhysAddr = MappedAddr(mboot_info as *const _ as usize).to_phys();
    let mboot_end:      PhysAddr = mboot_start + mboot_info.size as usize;
    let modules_start:  PhysAddr = mboot_info.modules_start_addr().unwrap_or_default();
    let modules_end:    PhysAddr = mboot_info.modules_end_addr().unwrap_or_default();

    let iter = PhysMemIterator::new(
        mm_iter,
        kern_start, kern_end, mboot_start, mboot_end, modules_start, modules_end);

    let mut head: Option<PhysAddr> = None;
    let mut tail: Option<PhysAddr> = None;

    for el in iter {
        if let Some(p) = tail {
            unsafe {
                p.to_mapped().store(el);
            }
        }

        if head.is_none() {
            head = Some(el);
        }

        tail = Some(el);
    }

    if let Some(p) = tail {
        unsafe {
            p.to_mapped().store(LIST_ADDR_INVALID);
        }

    }

    let mut l = PHYS_LIST.lock();

    if let Some(f) = head {
        l.head = f;
    }
}
