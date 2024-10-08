use crate::arch::mm::PAGE_SIZE;
use crate::kernel::mm::{MappedAddr, PhysAddr};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    pub fn new(address: PhysAddr) -> Frame {
        Frame {
            number: (address / PhysAddr(PAGE_SIZE)).0,
        }
    }

    pub fn clear(&mut self) {
        type Slice = [u8; PAGE_SIZE];
        unsafe { self.address_mapped().read_mut::<Slice>() }.fill(0)
    }

    pub fn address(&self) -> PhysAddr {
        PhysAddr(self.number) * PAGE_SIZE
    }

    pub fn address_mapped(&self) -> MappedAddr {
        self.address().to_mapped()
    }

    pub fn end_address(&self) -> PhysAddr {
        PhysAddr(self.number) * PAGE_SIZE + PAGE_SIZE
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn next(&self) -> Frame {
        Frame {
            number: self.number + 1,
        }
    }
}
