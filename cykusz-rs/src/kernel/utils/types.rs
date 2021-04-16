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

align_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
ceil_div_impl!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize u128 i128);
