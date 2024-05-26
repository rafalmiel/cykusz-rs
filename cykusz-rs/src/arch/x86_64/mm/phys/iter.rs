#![allow(unused)]

use crate::drivers::multiboot2::memory::MemoryIter;
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::mm::PhysAddr;

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

pub struct RangeMemIterator {
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

fn contains(saddr: PhysAddr, start: PhysAddr, end: PhysAddr) -> bool {
    !not_contains(saddr, start, end)
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

impl RangeMemIterator {
    pub fn new(
        mut mm_iter: MemoryIter,
        kern_start: PhysAddr,
        kern_end: PhysAddr,
        mboot_start: PhysAddr,
        mboot_end: PhysAddr,
        modules_start: PhysAddr,
        modules_end: PhysAddr,
    ) -> RangeMemIterator {
        let ent = mm_iter
            .next()
            .expect("Memory iterator needs at least one value");

        RangeMemIterator {
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

    fn adjust_up(&self, mut addr: PhysAddr) -> PhysAddr {
        if addr < PhysAddr(0x100000) {
            addr = PhysAddr(0x100000);
        }

        if contains(addr, self.kern_start, self.kern_end) {
            return self.kern_end;
        } else if contains(addr, self.mboot_start, self.mboot_end) {
            return self.mboot_end;
        } else if contains(addr, self.modules_start, self.modules_end) {
            return self.modules_end;
        } else {
            return addr;
        }
    }

    fn adjust_down(&self, addr: PhysAddr, start: PhysAddr) -> Option<PhysAddr> {
        if start >= addr {
            return None;
        }

        if self.kern_end < addr && self.kern_start >= start {
            return Some(self.kern_start);
        } else if self.mboot_end < addr && self.mboot_start >= start {
            return Some(self.mboot_start);
        } else if self.modules_end < addr && self.modules_start >= start {
            return Some(self.modules_start);
        } else {
            return Some(addr);
        }
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

impl Iterator for RangeMemIterator {
    type Item = (PhysAddr, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.mm_start >= self.mm_end {
            if let Some(e) = self.mm_iter.next() {
                self.mm_start = PhysAddr(e.base_addr as usize).align_up(0x1000);
                self.mm_end = PhysAddr(e.base_addr as usize) + e.length as usize;
            } else {
                return None;
            }
        }

        self.mm_start = self.adjust_up(self.mm_start).align_up(0x1000);
        if let Some(end) = self.adjust_down(self.mm_end, self.mm_start) {
            let ret = Some((self.mm_start, end.0 - self.mm_start.0));

            self.mm_start = end.align_up(0x1000);

            ret
        } else {
            return self.next();
        }
    }
}
