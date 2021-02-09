use crate::kernel::mm::PhysAddr;

pub mod address;
pub mod elf;
pub mod memory;
pub mod modules;

#[repr(C)]
pub struct Tag {
    pub typ: u32,
    pub size: u32,
}

pub struct TagIter {
    pub current: *const Tag,
}

impl Iterator for TagIter {
    type Item = &'static Tag;

    fn next(&mut self) -> Option<&'static Tag> {
        match unsafe { &*self.current } {
            &Tag { typ: 0, size: 8 } => None,
            tag => {
                self.current = (PhysAddr(self.current as usize) + tag.size as usize)
                    .align_up(8)
                    .0 as *const _;

                Some(tag)
            }
        }
    }
}
