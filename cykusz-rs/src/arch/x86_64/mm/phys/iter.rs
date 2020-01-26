use crate::drivers::multiboot2::memory::MemoryIter;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::PAGE_SIZE;

pub struct PhysMemIterator {
    current: PhysAddr,
    mm_iter: MemoryIter,
    mm_start: PhysAddr,
    mm_end: PhysAddr,
    kern_start: PhysAddr,
    kern_end: PhysAddr,
    mboot_start: PhysAddr,
    mboot_end: PhysAddr,
    modules_start: PhysAddr,
    modules_end: PhysAddr,
}

fn not_contains(saddr: PhysAddr, start: PhysAddr, end: PhysAddr) -> bool {
    let eaddr = saddr + PAGE_SIZE - 1;

    (saddr < start && eaddr < start) || (saddr >= end && eaddr >= end)
}

impl PhysMemIterator {
    pub fn new(
        mut mm_iter: MemoryIter,
        kern_start: PhysAddr,
        kern_end: PhysAddr,
        mboot_start: PhysAddr,
        mboot_end: PhysAddr,
        modules_start: PhysAddr,
        modules_end: PhysAddr,
    ) -> PhysMemIterator {
        let ent = mm_iter
            .next()
            .expect("Memory iterator needs at least one value");

        PhysMemIterator {
            current: PhysAddr(ent.base_addr as usize),
            mm_iter,
            mm_start: PhysAddr(ent.base_addr as usize),
            mm_end: PhysAddr(ent.base_addr as usize) + ent.length as usize,
            kern_start,
            kern_end,
            mboot_start,
            mboot_end,
            modules_start,
            modules_end,
        }
    }

    fn is_valid(&self, addr: PhysAddr) -> bool {
        not_contains(addr, self.kern_start, self.kern_end)
            && not_contains(addr, self.mboot_start, self.mboot_end)
            && not_contains(addr, self.modules_start, self.modules_end)
            && addr >= PhysAddr(0x100000)
    }
}

impl Iterator for PhysMemIterator {
    type Item = PhysAddr;

    fn next(&mut self) -> Option<PhysAddr> {
        loop {
            let c = self.current;

            if c >= self.mm_end {
                if let Some(e) = self.mm_iter.next() {
                    self.mm_start = PhysAddr(e.base_addr as usize);
                    self.mm_end = PhysAddr(e.base_addr as usize) + e.length as usize;
                    self.current = self.mm_start;
                    continue;
                } else {
                    return None;
                }
            }

            self.current += PAGE_SIZE;

            if self.is_valid(c) {
                return Some(c);
            }
        }
    }
}
