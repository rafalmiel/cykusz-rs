#![allow(dead_code)]

#[derive(Debug, Default, Copy, Clone)]
pub struct NetU32(u32);
#[derive(Debug, Default, Copy, Clone)]
pub struct NetU16(u16);
#[derive(Debug, Default, Copy, Clone)]
pub struct NetU8(u8);

macro_rules! impl_net (
    ($type_: ident, $src: ident) => {
        impl $type_ {
            pub const fn new(v: $src) -> $type_ {
                if cfg!(target_endian = "little") {
                    $type_(v.swap_bytes())
                } else {
                    $type_(v)
                }
            }

            pub const fn value(self) -> $src {
                if cfg!(target_endian = "little") {
                    self.0.swap_bytes()
                } else {
                    self.0
                }
            }

            pub const fn net_value(self) -> $src {
                self.0
            }
        }
    }
);

impl_net!(NetU32, u32);
impl_net!(NetU16, u16);
impl_net!(NetU8, u8);
