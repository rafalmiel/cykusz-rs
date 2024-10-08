use bit_field::BitField;

use crate::kernel::mm::PhysAddr;

use super::bump;

type BMType = u64;
const BMTYPE_BITS: usize = core::mem::size_of::<BMType>() * 8;
pub const BUDDY_COUNT: usize = 7;

pub struct BuddyAlloc {
    start: PhysAddr,
    end: PhysAddr,
    buddies: [&'static mut [BMType]; BUDDY_COUNT],
    freecnt: [usize; BUDDY_COUNT],
    ranges: [(PhysAddr, PhysAddr); 16],
    num_ranges: usize,
}

pub static BSIZE: [usize; BUDDY_COUNT] =
    [0x1000, 0x2000, 0x4000, 0x8000, 0x10000, 0x20000, 0x40000];

impl BuddyAlloc {
    pub const fn new() -> BuddyAlloc {
        BuddyAlloc {
            start: PhysAddr(0),
            end: PhysAddr(0),
            buddies: [
                &mut [],
                &mut [],
                &mut [],
                &mut [],
                &mut [],
                &mut [],
                &mut [],
            ],
            freecnt: [0usize; BUDDY_COUNT],
            ranges: [(PhysAddr(0), PhysAddr(0)); 16],
            num_ranges: 0,
        }
    }

    pub fn init(&mut self, start: PhysAddr, end: PhysAddr) {
        self.start = start;
        self.end = end;

        let size = (self.end - self.start).0;

        for (idx, bs) in BSIZE.iter().enumerate() {
            let s = ((size / bs) + BMTYPE_BITS - 1) / BMTYPE_BITS;
            let als = s * (BMTYPE_BITS / 8);

            let slice = unsafe { bump::alloc(als).as_slice_mut::<BMType>(s) };
            slice.fill(0);

            self.buddies[idx] = slice;
        }
    }

    fn find_ord(&self, addr: PhysAddr, max_size: usize) -> usize {
        for ord in (0..BSIZE.len()).rev() {
            let size = BSIZE[ord];

            if size > max_size {
                continue;
            }

            let mask = BSIZE[ord] - 1;

            if mask & addr.0 != 0 {
                continue;
            } else {
                return ord;
            }
        }

        return 0;
    }

    pub fn add_range(&mut self, start: PhysAddr, end: PhysAddr) {
        assert!(start >= self.start && end <= self.end);
        assert!(self.num_ranges < self.ranges.len());

        let mut rem = (end - start).0;

        let mut cur = start;

        while rem > 0 {
            let ord = self.find_ord(cur, rem);
            let size = BSIZE[ord];

            assert!(self.set_bit(cur, ord));

            cur += size;
            rem -= size;
        }

        self.ranges[self.num_ranges] = (start, end);
        self.num_ranges += 1;
    }

    fn find_free(&mut self, idx: usize) -> PhysAddr {
        for (i, el) in self.buddies[idx].iter_mut().enumerate() {
            let mut v: BMType = *el;

            if v != 0 {
                let mut ib = 0;

                while !v.get_bit(0) {
                    v >>= 1;
                    ib += 1;
                }

                assert!((*el).get_bit(ib));

                (*el).set_bit(ib, false);

                self.freecnt[idx] -= 1;

                return self.start.align_up(BSIZE[idx])
                    + (BSIZE[idx] * BMTYPE_BITS * i)
                    + BSIZE[idx] * ib;
            }
        }

        panic!("Unexpected!!!");
    }

    pub fn alloc(&mut self, order: usize) -> Option<PhysAddr> {
        let size = BSIZE[order];

        for (i, &s) in BSIZE[order..].iter().enumerate() {
            let i = i + order;

            if self.freecnt[i] > 0 {
                let res = self.find_free(i);

                let mut rem = s - size;

                if rem > 0 {
                    for ri in (0..=i).rev() {
                        let si = BSIZE[ri];

                        if rem >= si {
                            assert!(self.set_bit(res + (rem - si) + size, ri));

                            rem -= si;
                        }
                    }
                }
                return Some(res);
            }
        }

        None
    }

    fn get_byte_bit(&self, addr: PhysAddr, order: usize) -> (usize, usize) {
        let offset = (addr - self.start).0;

        let id = offset / BSIZE[order];

        (id / BMTYPE_BITS, id % BMTYPE_BITS)
    }

    #[allow(unused)]
    fn is_set(&self, addr: PhysAddr, order: usize) -> bool {
        let (byte, bit) = self.get_byte_bit(addr, order);

        self.buddies[order][byte].get_bit(bit) == true
    }

    #[allow(unused)]
    fn is_clear(&self, addr: PhysAddr, order: usize) -> bool {
        let (byte, bit) = self.get_byte_bit(addr, order);

        self.buddies[order][byte].get_bit(bit) == false
    }

    fn set_bit(&mut self, addr: PhysAddr, order: usize) -> bool {
        let (byte, bit) = self.get_byte_bit(addr, order);

        let b = &mut self.buddies[order][byte];

        let change = (*b).get_bit(bit) == false;

        if change {
            (*b).set_bit(bit, true);

            self.freecnt[order] += 1;
        }

        change
    }

    fn clear_bit(&mut self, addr: PhysAddr, order: usize) -> bool {
        if addr < self.start {
            return false;
        }

        let (byte, bit) = self.get_byte_bit(addr, order);

        let b = &mut self.buddies[order][byte];

        let change = (*b).get_bit(bit) == true;

        if change {
            (*b).set_bit(bit, false);

            self.freecnt[order] -= 1;
        }

        change
    }

    fn get_buddy(&self, addr: PhysAddr, order: usize) -> PhysAddr {
        let size = BSIZE[order];

        let base = addr.align_down(size * 2);

        if base == addr {
            addr + size
        } else {
            base
        }
    }

    fn in_range(&self, addr: PhysAddr, order: usize) -> bool {
        let end = addr + BSIZE[order];

        self.ranges[..self.num_ranges]
            .iter()
            .find(|(s, e)| *s <= addr && *e > end)
            .is_some()
    }

    pub fn dealloc(&mut self, mut addr: PhysAddr, mut order: usize) {
        if !self.in_range(addr, order) {
            return;
        }
        while order < BSIZE.len() {
            if order < BSIZE.len() - 1 {
                let buddy = self.get_buddy(addr, order);

                if self.clear_bit(buddy, order) {
                    // merge
                    addr = core::cmp::min(addr, buddy);
                    order += 1;
                } else {
                    self.set_bit(addr, order);
                    break;
                }
            } else {
                self.set_bit(addr, order);
                break;
            }
        }
    }

    pub fn used_mem(&self) -> usize {
        (self.end - self.start).0 - self.free_mem()
    }

    pub fn free_mem(&self) -> usize {
        self.freecnt
            .iter()
            .enumerate()
            .fold(0, |acc, (i, e)| acc + (*e * BSIZE[i]))
    }
}
