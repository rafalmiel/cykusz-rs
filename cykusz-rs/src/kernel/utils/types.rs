use crate::kernel::mm::PAGE_SIZE;

pub trait Align: Sized {
    fn align(self, align: Self) -> Self {
        self.align_down(align)
    }

    fn align_up(self, align: Self) -> Self;

    fn align_down(self, align: Self) -> Self;
}

pub trait CeilDiv {
    fn ceil_div(self, d: Self) -> Self;
}

macro_rules! align_impl {
    ($($t:ty)*) => ($(
        impl Align for $t {
            fn align_down(self, align: $t) -> $t {
                if (align as usize).is_power_of_two() {
                    self & (!(align - 1))
                } else if align == 0 {
                    self
                } else {
                    self - (self % align)
                }
            }

            fn align_up(self, align: $t) -> $t {
                (self + align - 1).align(align)
            }
        }
    )*)
}

macro_rules! ceil_div_impl {
    ($($t:ty)*) => ($(
        impl CeilDiv for $t {
            fn ceil_div(self, d: $t) -> $t {
                (self + d - 1) / d
            }
        }
    )*)
}

struct PageIter<'a, T> {
    data: &'a [T],
    cur: Option<&'a T>,
}

impl<'a, T> PageIter<'a, T> {
    fn new(data: &'a [T]) -> PageIter<'a, T> {
        PageIter::<'a, T> {
            data,
            cur: data.first(),
        }
    }
}

impl<'a, T> Iterator for PageIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.cur?;
        let ret = c;

        self.cur = {
            let last = self.data.last()? as *const T;

            let next = unsafe {
                ((c as *const _ as *const u8).offset(PAGE_SIZE as isize) as usize)
                    .align_down(PAGE_SIZE) as *const T
            };

            if next <= last {
                unsafe { Some(&*next) }
            } else {
                None
            }
        };

        Some(ret)
    }
}

pub trait Prefault {
    fn prefault(&self);
}

impl<T: Copy> Prefault for &T {
    fn prefault(&self) {
        let t = **self;

        core::hint::black_box(t); // prevent optimisations...
    }
}

impl Prefault for &[u8] {
    fn prefault(&self) {
        // Reads first byte of each page of the buffer to fetch it into memory
        for d in PageIter::new(self) {
            d.prefault();
        }
    }
}
impl Prefault for &mut [u8] {
    fn prefault(&self) {
        (self as &[u8]).prefault()
    }
}

align_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
ceil_div_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
