use crate::kernel::mm::virt;
use crate::kernel::mm::Frame;
use crate::kernel::mm::MappedAddr;
use crate::kernel::mm::PhysAddr;

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
pub const COUNTER_MASK: usize = 0x3ff0_0000_0000_0000;

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
        self.bits = frame.address().0;
        self.insert(flags);
    }

    pub fn set_frame(&mut self, frame: &Frame) {
        self.bits = frame.address().0;
    }

    pub fn set_flags(&mut self, flags: Entry) {
        self.insert(flags);
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
