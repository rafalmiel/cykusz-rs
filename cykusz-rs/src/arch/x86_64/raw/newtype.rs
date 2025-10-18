macro_rules! simple_impl2 {
    (impl $trait_: ident for $type_: ident { fn $method: ident }) => {
        impl $trait_<$type_> for $type_ {
            type Output = $type_;

            fn $method(self, $type_(b): $type_) -> $type_ {
                let $type_(a) = self;
                $type_(a.$method(&b))
            }
        }
    };
    (impl $trait_: ident for $type_: ident { fn $method: ident } with $type2_:ident) => {
        impl $trait_<$type2_> for $type_ {
            type Output = $type_;

            fn $method(self, b: $type2_) -> $type_ {
                let $type_(a) = self;
                $type_(a.$method(&b))
            }
        }
    };
}

macro_rules! simple_impl2_assign {
    (impl $trait_: ident for $type_: ident { fn $method: ident }) => {
        impl $trait_<$type_> for $type_ {
            fn $method(&mut self, $type_(b): $type_) {
                (self.0).$method(b);
            }
        }
    };
    (impl $trait_: ident for $type_: ident { fn $method: ident } with $type2_:ident) => {
        impl $trait_<$type2_> for $type_ {
            fn $method(&mut self, b: $type2_) {
                (self.0).$method(b);
            }
        }
    };
}

macro_rules! simple_impl1 {
    (impl $trait_: ident for $type_: ident { fn $method: ident }) => {
        impl $trait_ for $type_ {
            type Output = $type_;

            fn $method(self) -> $type_ {
                let $type_(a) = self;
                $type_(a.$method())
            }
        }
    };
}

macro_rules! simple_from {
    (from $from_: ident for $type_: ident) => {
        impl From<$from_> for $type_ {
            fn from(v: $from_) -> $type_ {
                $type_(v as usize)
            }
        }
    };
}

macro_rules! enable_unsigned_ops {
    ($type_: ident) => {
        simple_impl2! { impl Add for $type_ { fn add } }
        simple_impl2! { impl Add for $type_ { fn add } with usize }
        simple_impl2! { impl Sub for $type_ { fn sub } }
        simple_impl2! { impl Sub for $type_ { fn sub } with usize }
        simple_impl2! { impl Mul for $type_ { fn mul } }
        simple_impl2! { impl Mul for $type_ { fn mul } with usize }
        simple_impl2! { impl Div for $type_ { fn div } }
        simple_impl2! { impl Div for $type_ { fn div } with usize }
        simple_impl2! { impl Rem for $type_ { fn rem } }
        simple_impl2! { impl Rem for $type_ { fn rem } with usize }
        simple_impl2_assign! { impl AddAssign for $type_ { fn add_assign } }
        simple_impl2_assign! { impl AddAssign for $type_ { fn add_assign } with usize }
        simple_impl2_assign! { impl SubAssign for $type_ { fn sub_assign } }
        simple_impl2_assign! { impl SubAssign for $type_ { fn sub_assign } with usize }
        simple_impl2_assign! { impl MulAssign for $type_ { fn mul_assign } }
        simple_impl2_assign! { impl MulAssign for $type_ { fn mul_assign } with usize }
        simple_impl2_assign! { impl DivAssign for $type_ { fn div_assign } }
        simple_impl2_assign! { impl DivAssign for $type_ { fn div_assign } with usize }
        simple_impl2_assign! { impl RemAssign for $type_ { fn rem_assign } }
        simple_impl2_assign! { impl RemAssign for $type_ { fn rem_assign } with usize }

        simple_impl1! { impl Not for $type_ { fn not } }
        simple_impl2! { impl BitAnd for $type_ { fn bitand } }
        simple_impl2! { impl BitAnd for $type_ { fn bitand } with usize }
        simple_impl2! { impl BitOr for $type_ { fn bitor } }
        simple_impl2! { impl BitOr for $type_ { fn bitor } with usize }
        simple_impl2! { impl BitXor for $type_ { fn bitxor } }
        simple_impl2! { impl BitXor for $type_ { fn bitxor } with usize }
        simple_impl2! { impl Shl for $type_ { fn shl } }
        simple_impl2! { impl Shl for $type_ { fn shl } with usize }
        simple_impl2! { impl Shr for $type_ { fn shr } }
        simple_impl2! { impl Shr for $type_ { fn shr } with usize }

        simple_impl2_assign! { impl BitAndAssign for $type_ { fn bitand_assign } }
        simple_impl2_assign! { impl BitAndAssign for $type_ { fn bitand_assign } with usize }
        simple_impl2_assign! { impl BitOrAssign for $type_ { fn bitor_assign } }
        simple_impl2_assign! { impl BitOrAssign for $type_ { fn bitor_assign } with usize }
        simple_impl2_assign! { impl BitXorAssign for $type_ { fn bitxor_assign } }
        simple_impl2_assign! { impl BitXorAssign for $type_ { fn bitxor_assign } with usize }
        simple_impl2_assign! { impl ShlAssign for $type_ { fn shl_assign } }
        simple_impl2_assign! { impl ShrAssign for $type_ { fn shr_assign } }

        simple_from!( from u64 for $type_);
        simple_from!( from u32 for $type_);
        simple_from!( from u16 for $type_);
        simple_from!( from u8 for $type_);

        impl ::core::fmt::Display for $type_ {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                write!(f, "0x{:x}", self.0)
            }
        }

        impl $type_ {
            pub fn align_down(self, align: usize) -> $type_ {
                if align.is_power_of_two() {
                    $type_(self.0 & !(align - 1))
                } else if align == 0 {
                    self
                } else {
                    panic!("`align` must be a power of 2");
                }
            }

            pub fn align_up(self, align: usize) -> $type_ {
                let v = $type_(self.0 + align - 1);
                v.align_down(align)
            }

            pub fn align(self, align: usize) -> $type_ {
                self.align_down(align)
            }
        }

        impl $type_ {
            pub unsafe fn store<T: Copy>(self, v: T) { unsafe {
                *(self.0 as *mut T) = v;
            }}

            pub unsafe fn store_volatile<T: Copy>(self, v: T) { unsafe {
                core::ptr::write_volatile(self.0 as *mut T, v);
            }}

            pub unsafe fn read<T: Copy>(self) -> T { unsafe {
                return *(self.0 as *mut T);
            }}

            pub unsafe fn read_volatile<T: Copy>(self) -> T { unsafe {
                return ::core::ptr::read_volatile(self.0 as *const T);
            }}

            pub unsafe fn read_ref<'a, T>(self) -> &'a T { unsafe {
                return &*(self.0 as *mut T);
            }}

            pub unsafe fn read_mut<'a, T>(self) -> &'a mut T { unsafe {
                return &mut *(self.0 as *mut T);
            }}

            pub unsafe fn copy_to(self, to: usize, count: usize) { unsafe {
                (self.0 as *const u8).copy_to(to as *mut u8, count);
            }}

            pub unsafe fn copy_page_from(self, src: $type_) { unsafe {
                self.align_down(PAGE_SIZE).as_bytes_mut(PAGE_SIZE).copy_from_slice(src.align_down(PAGE_SIZE).as_bytes(PAGE_SIZE));
            }}

            pub unsafe fn copy_page_from_bytes(self, src: $type_, bytes: usize) { unsafe {
                self.align_down(PAGE_SIZE).as_bytes_mut(bytes).copy_from_slice(src.align_down(PAGE_SIZE).as_bytes(bytes));
            }}

            pub unsafe fn as_bytes<'a>(self, size: usize) -> &'a [u8] { unsafe {
                core::slice::from_raw_parts(self.0 as *const u8, size)
            }}

            pub unsafe fn as_bytes_mut<'a>(self, size: usize) -> &'a mut [u8] { unsafe {
                core::slice::from_raw_parts_mut(self.0 as *mut u8, size)
            }}

            pub unsafe fn as_slice<'a, T>(self, count: usize) -> &'a [T] { unsafe {
                core::slice::from_raw_parts(self.0 as *const T, count)
            }}

            pub unsafe fn as_slice_mut<'a, T>(self, count: usize) -> &'a mut [T] { unsafe {
                core::slice::from_raw_parts_mut(self.0 as *mut T, count)
            }}
        }

        impl ::core::iter::Step for $type_ {
            /// Returns the number of steps between two step objects. The count is
            /// inclusive of `start` and exclusive of `end`.
            ///
            /// Returns `None` if it is not possible to calculate `steps_between`
            /// without overflow.
            fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
                if start <= end {
                    let diff = end.0 - start.0;
                    return (diff, Some(diff));
                } else {
                    (0, None)
                }
            }

            fn forward_checked(start: Self, count: usize) -> Option<Self> {
                Some($type_(start.0 + count))
            }

            fn backward_checked(start: Self, count: usize) -> Option<Self> {
                Some($type_(start.0 - count))
            }
/*
            /// Replaces this step with `1`, returning itself
            fn replace_one(&mut self) -> Self {
                ::core::mem::replace(self, $type_(1))
            }

            /// Replaces this step with `0`, returning itself
            fn replace_zero(&mut self) -> Self {
                ::core::mem::replace(self, $type_(0))
            }

            /// Adds one to this step, returning the result
            fn add_one(&self) -> Self {
                $type_(self.0 + 1)
            }

            /// Subtracts one to this step, returning the result
            fn sub_one(&self) -> Self {
                $type_(self.0 - 1)
            }

            /// Add an usize, returning None on overflow
            fn add_usize(&self, n: usize) -> Option<Self> {
                Some($type_(self.0 + n))
            }
            */
        }
    }
}
