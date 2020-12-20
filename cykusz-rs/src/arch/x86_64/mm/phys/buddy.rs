use bit_field::BitField;

use crate::kernel::mm::PhysAddr;

use super::bump;

type BMType = u64;
const BMTYPE_BITS: usize = core::mem::size_of::<BMType>() * 8;

pub struct BuddyAlloc {
    start: PhysAddr,
    end: PhysAddr,
    buddies: [&'static mut [BMType]; 6],
    freecnt: [usize; 6],
}

pub static BSIZE: [usize; 6] = [0x1000, 0x2000, 0x4000, 0x8000, 0x10000, 0x20000];

impl BuddyAlloc {
    pub const fn new() -> BuddyAlloc {
        BuddyAlloc {
            start: PhysAddr(0),
            end: PhysAddr(0),
            buddies: [&mut [], &mut [], &mut [], &mut [], &mut [], &mut []],
            freecnt: [0usize; 6],
        }
    }

    pub fn init(&mut self, start: PhysAddr, end: PhysAddr) {
        self.start = start;
        self.end = end;

        let size = (self.end - self.start).0;

        for (idx, bs) in BSIZE.iter().enumerate() {
            let s = ((size / bs) + BMTYPE_BITS - 1) / BMTYPE_BITS;
            let als = s * (BMTYPE_BITS / 8);

            let ptr = bump::alloc(als).0 as *mut BMType;

            self.buddies[idx] = unsafe { core::slice::from_raw_parts_mut(ptr, s) };
            self.buddies[idx].fill(0);
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

        let mut rem = (end - start).0;

        let mut cur = start;

        while rem > 0 {
            let ord = self.find_ord(cur, rem);
            let size = BSIZE[ord];

            self.set_bit(cur, ord);

            cur += size;
            rem -= size;
        }
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

        for (i, s) in BSIZE[order..].iter().enumerate() {
            let i = i + order;

            if *s >= size && self.freecnt[i] > 0 {
                let res = self.find_free(i);

                let mut rem = *s - size;

                if rem > 0 {
                    for ri in (0..=i).rev() {
                        let si = BSIZE[ri];

                        if rem >= si {
                            self.set_bit(res + (rem - si) + size, ri);

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
        let dbl = size * 2;

        let offset = (addr - self.start + size).0;

        if offset % dbl == 0 {
            addr - size
        } else if offset % dbl == size {
            addr + size
        } else {
            panic!("BuddyAlloc get_buddy BUG!");
        }
    }

    pub fn dealloc(&mut self, mut addr: PhysAddr, mut order: usize) {
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
}
