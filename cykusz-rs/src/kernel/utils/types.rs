pub trait Align {
    fn align(self, align: Self) -> Self;

    fn align_up(self, align: Self) -> Self;
}

pub trait CeilDiv {
    fn ceil_div(self, d: Self) -> Self;
}

macro_rules! align_impl {
    ($($t:ty)*) => ($(
        impl Align for $t {
            fn align(self, align: $t) -> $t {
                if (align as usize).is_power_of_two() {
                    self & (!(align - 1))
                } else if align == 0 {
                    self
                } else {
                    panic!("`align` must be a power of 2");
                }
            }

            fn align_up(self, align: $t) -> $t {
                self.align(self + align - 1)
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

align_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
ceil_div_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
