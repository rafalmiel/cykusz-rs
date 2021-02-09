use crate::kernel::mm::Frame;
use crate::kernel::mm::MappedAddr;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::{deallocate_order, virt};

bitflags! {
    pub struct Entry: usize {
        const PRESENT       = 1 << 0;
        const WRITABLE      = 1 << 1;
        const USER          = 1 << 2;
        const WRT_THROUGH   = 1 << 3;
        const NO_CACHE      = 1 << 4;
        const ACCESSED      = 1 << 5;
        const DIRTY         = 1 << 6;
        const HUGE_PAGE     = 1 << 7;
        const GLOBAL        = 1 << 8;
        const NO_EXECUTE    = 1 << 63;
    }
}

pub const ADDRESS_MASK: usize = 0x000f_ffff_ffff_f000;
pub const COUNTER_MASK: usize = 0x7ff0_0000_0000_0000;
pub const FLAG_MASK: usize = 0x80000000000001FF;

impl Entry {
    pub fn new_empty() -> Entry {
        Entry { bits: 0 }
    }

    pub fn from_kernel_flags(flags: virt::PageFlags) -> Entry {
        let mut res: Entry = Entry::new_empty();

        if flags.contains(virt::PageFlags::NO_EXECUTE) {
            res.insert(Entry::NO_EXECUTE);
        }
        if flags.contains(virt::PageFlags::USER) {
            res.insert(Entry::USER);
        }
        if flags.contains(virt::PageFlags::WRITABLE) {
            res.insert(Entry::WRITABLE);
        }
        if flags.contains(virt::PageFlags::NO_CACHE) {
            res.insert(Entry::NO_CACHE);
        }
        if flags.contains(virt::PageFlags::WRT_THROUGH) {
            res.insert(Entry::WRT_THROUGH);
        }

        return res;
    }

    pub unsafe fn from_addr(addr: MappedAddr) -> Entry {
        Entry {
            bits: addr.read::<usize>(),
        }
    }

    pub fn clear(&mut self) {
        self.bits = 0;
    }

    pub fn raw(&self) -> usize {
        self.bits
    }

    pub fn set_raw(&mut self, bits: usize) {
        self.bits = bits;
    }

    pub fn is_unused(&self) -> bool {
        self.bits == 0
    }

    pub fn address(&self) -> PhysAddr {
        PhysAddr(self.bits & ADDRESS_MASK)
    }

    pub fn frame(&self) -> Option<Frame> {
        if self.contains(Entry::PRESENT) {
            Some(Frame::new(self.address()))
        } else {
            None
        }
    }

    pub fn set_frame_flags(&mut self, frame: &Frame, flags: Entry) {
        //println!("set frame flags {} {:?}", frame.address(), flags);
        self.set_frame(frame);
        self.set_flags(flags);
    }

    pub fn unref_phys_page(&self) -> bool {
        if self.address() != PhysAddr(0) {
            if let Some(page) = self.address().to_phys_page() {
                let cnt = page.dec_vm_use_count();
                if cnt == 0 {
                    //println!("unref phys page dealloc {}", self.address());
                    deallocate_order(&Frame::new(self.address()), 0);

                    return true;
                }
            }
        }

        false
    }

    pub fn ref_phys_page(&self) {
        if self.address() != PhysAddr(0) {
            if let Some(page) = self.address().to_phys_page() {
                page.inc_vm_use_count();
            }
        }
    }

    pub fn set_frame(&mut self, frame: &Frame) {
        let ref_page = self.address() != frame.address();
        //println!("set frame {} do ref page? {}", frame.address(), ref_page);

        if ref_page {
            self.unref_phys_page();
        }

        self.bits &= !ADDRESS_MASK;
        self.bits |= frame.address().0;

        if ref_page {
            self.ref_phys_page();
        }
    }

    pub fn set_flags(&mut self, flags: Entry) {
        //println!("set flags {} {:?}", self.address(), flags);
        self.bits &= !FLAG_MASK;
        self.insert(Entry::from_bits(flags.bits & FLAG_MASK).unwrap());
    }

    pub fn get_entry_count(&self) -> usize {
        (self.bits & COUNTER_MASK) >> 52
    }

    pub fn set_entry_count(&mut self, count: usize) {
        self.bits = (self.bits & !COUNTER_MASK) | (count << 52);
    }

    pub fn inc_entry_count(&mut self) {
        let c = self.get_entry_count();
        self.set_entry_count(c + 1);
    }

    pub fn dec_entry_count(&mut self) {
        let c = self.get_entry_count();

        assert_ne!(c, 0);

        self.set_entry_count(c - 1);
    }
}
