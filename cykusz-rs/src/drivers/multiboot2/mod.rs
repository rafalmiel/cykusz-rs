use crate::kernel::mm::{MappedAddr, PhysAddr, VirtAddr};

pub use self::tags::*;

pub mod tags;

#[repr(C)]
pub struct Info {
    pub size: u32,
    reserved: u32,
    pub tag: tags::Tag,
}

pub unsafe fn load(addr: MappedAddr) -> &'static Info {
    &*(addr.0 as *const Info)
}

impl Info {
    pub fn kernel_start_addr(&self) -> PhysAddr {
        self.elf_tag()
            .unwrap()
            .sections()
            .nth(0)
            .unwrap()
            .address()
            .to_phys()
    }

    pub fn kernel_end_addr(&self) -> PhysAddr {
        let item = self.elf_tag().unwrap().sections().last().unwrap();

        item.address().to_phys() + item.size as usize
    }

    pub fn modules_start_addr(&self) -> Option<PhysAddr> {
        self.modules_tags()
            .nth(0)
            .and_then(|m| Some(VirtAddr(m.mod_start as usize).to_phys()))
    }

    pub fn modules_end_addr(&self) -> Option<PhysAddr> {
        self.modules_tags()
            .last()
            .and_then(|m| Some(VirtAddr(m.mod_end as usize).to_phys()))
    }

    pub fn tags(&self) -> tags::TagIter {
        tags::TagIter {
            current: &self.tag as *const _,
        }
    }

    pub fn memory_map_tag(&self) -> Option<&'static tags::memory::Memory> {
        self.tags()
            .find(|t| t.typ == 6)
            .map(|t| unsafe { &*(t as *const tags::Tag as *const tags::memory::Memory) })
    }

    pub fn address_tag(&self) -> Option<&'static tags::address::Address> {
        self.tags()
            .find(|t| t.typ == 2)
            .map(|t| unsafe { &*(t as *const tags::Tag as *const tags::address::Address) })
    }

    pub fn elf_tag(&self) -> Option<&'static tags::elf::Elf> {
        self.tags()
            .find(|t| t.typ == 9)
            .map(|t| unsafe { &*(t as *const tags::Tag as *const tags::elf::Elf) })
    }

    pub fn command_line_tag(&self) -> Option<&'static tags::command_line::CommandLine> {
        self.tags()
            .find(|t| t.typ == 1)
            .map(|t| unsafe { &*(t as *const tags::Tag as *const tags::command_line::CommandLine) })
    }

    pub fn framebuffer_info_tag(&self) -> Option<&'static tags::framebuffer_info::FramebufferInfo> {
        self.tags().find(|t| t.typ == 8).map(|t| unsafe {
            &*(t as *const tags::Tag as *const tags::framebuffer_info::FramebufferInfo)
        })
    }

    pub fn modules_tags(
        &self,
    ) -> ::core::iter::FilterMap<
        tags::TagIter,
        fn(&tags::Tag) -> Option<&'static tags::modules::Modules>,
    > {
        self.tags().filter_map(|t| {
            if t.typ == 3 {
                Some(unsafe { &*(t as *const tags::Tag as *const tags::modules::Modules) })
            } else {
                None
            }
        })
    }

    pub fn command_line_tags(
        &self,
    ) -> ::core::iter::FilterMap<
        tags::TagIter,
        fn(&tags::Tag) -> Option<&'static tags::command_line::CommandLine>,
    > {
        self.tags().filter_map(|t| {
            if t.typ == 1 {
                Some(unsafe { &*(t as *const tags::Tag as *const tags::command_line::CommandLine) })
            } else {
                None
            }
        })
    }
}
