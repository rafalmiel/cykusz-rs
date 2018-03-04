use core::ops::*;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct VirtAddr(pub usize);
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct PhysAddr(pub usize);
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct MappedAddr(pub usize);

enable_unsigned_ops!(VirtAddr);
enable_unsigned_ops!(PhysAddr);
enable_unsigned_ops!(MappedAddr);
