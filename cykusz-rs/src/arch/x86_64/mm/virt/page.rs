use crate::kernel::mm::VirtAddr;
use crate::kernel::mm::PAGE_SIZE;

pub struct Page {
    number: usize,
}

impl Page {
    pub fn new(virt: VirtAddr) -> Page {
        Page {
            number: virt.0 / PAGE_SIZE as usize,
        }
    }

    #[allow(unused)]
    pub fn address(&self) -> VirtAddr {
        VirtAddr(self.number) * PAGE_SIZE
    }

    pub fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }

    pub fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }

    pub fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }

    pub fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }
}
