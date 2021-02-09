use alloc::collections::linked_list::CursorMut;
use alloc::collections::LinkedList;
use alloc::string::String;

use crate::arch::mm::PAGE_SIZE;
use crate::drivers::elf::types::{ProgramFlags, ProgramType};
use crate::drivers::elf::ElfHeader;
use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::pcache::{PageCacheKey, PageItem};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, map_flags, map_to_flags, update_flags, VirtAddr};
use crate::kernel::sync::Spin;
use crate::kernel::utils::types::Align;

bitflags! {
    pub struct Prot: usize {
        const PROT_READ = 0x1;
        const PROT_WRITE = 0x2;
        const PROT_EXEC = 0x4;
        const PROT_NONE = 0x0;
    }
}

impl From<Prot> for PageFlags {
    fn from(e: Prot) -> Self {
        let mut res = PageFlags::empty();

        if !e.contains(Prot::PROT_EXEC) {
            res.insert(PageFlags::NO_EXECUTE);
        }

        if e.contains(Prot::PROT_WRITE) {
            res.insert(PageFlags::WRITABLE);
        }

        res
    }
}

impl From<ProgramFlags> for Prot {
    fn from(f: ProgramFlags) -> Self {
        let mut prot = Prot::empty();

        if f.contains(ProgramFlags::READABLE) {
            prot.insert(Prot::PROT_READ);
        }

        if f.contains(ProgramFlags::WRITABLE) {
            prot.insert(Prot::PROT_WRITE);
        }

        if f.contains(ProgramFlags::EXECUTABLE) {
            prot.insert(Prot::PROT_EXEC);
        }

        prot
    }
}

bitflags! {
    pub struct Flags: usize {
        const MAP_SHARED = 0x1;
        const MAP_PRIVATE = 0x2;
        const MAP_FIXED = 0x10;
        const MAP_ANONYOMUS = 0x20;
    }
}

#[derive(Clone)]
struct MMapedFile {
    file: DirEntryItem,
    length: usize,
    starting_offset: usize,
    active_mappings: hashbrown::HashMap<PageCacheKey, PageItem>,
}

impl MMapedFile {
    fn new(file: DirEntryItem, offset: usize, len: usize) -> MMapedFile {
        MMapedFile {
            file,
            length: len,
            starting_offset: offset,
            active_mappings: hashbrown::HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct Mapping {
    mmaped_file: Option<MMapedFile>,
    prot: Prot,
    flags: Flags,
    start: VirtAddr,
    end: VirtAddr,
}

impl Mapping {
    fn new(
        addr: VirtAddr,
        len: usize,
        prot: Prot,
        flags: Flags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Mapping {
        Mapping {
            mmaped_file: file.map(|e| MMapedFile::new(e, offset, len)),
            prot,
            flags,
            start: addr,
            end: addr + len,
        }
    }

    fn map_copy(addr: VirtAddr, src: VirtAddr, prot: Prot) {
        let new_page = allocate_order(0).unwrap();

        unsafe {
            new_page.address_mapped().as_virt().copy_page_from(src);
        }

        map_to_flags(addr, new_page.address(), PageFlags::USER | prot.into());
    }

    fn handle_pf_private_anon(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        let addr_aligned = addr.align_down(PAGE_SIZE);

        if !reason.contains(PageFaultReason::PRESENT) {
            // Page not present so just make it available
            map_flags(addr_aligned, PageFlags::USER | self.prot.into());

            true
        } else if reason.contains(PageFaultReason::WRITE) {
            if let Some(phys) = addr_aligned.to_phys_pagewalk() {
                if let Some(phys_page) = phys.to_phys_page() {
                    // If there is more than one process mapping this page, make a private copy
                    // Otherwise, this page is not shared with anyone, so just make it writable
                    if phys_page.vm_use_count() > 1 {
                        Self::map_copy(addr_aligned, addr_aligned, self.prot);
                    } else {
                        if !update_flags(addr_aligned, PageFlags::USER | self.prot.into()) {
                            panic!("Update flags failed");
                        }
                    };

                    return true;
                }
            }

            false
        } else {
            false
        }
    }

    fn handle_pf_private_file(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if let Some(f) = self.mmaped_file.as_mut() {
            let offset = (addr - self.start).0 + f.starting_offset;

            if let Some(p) = f.file.inode().as_cacheable().unwrap().get_mmap_page(offset) {
                if !reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    // Page is not present and we are reading from it, so map it readable
                    f.active_mappings.insert(p.cache_key(), p.clone());

                    map_to_flags(addr.align_down(PAGE_SIZE), p.page(), PageFlags::USER);
                } else if reason.contains(PageFaultReason::WRITE) {
                    // We are writing to private file mapping so copy the content of the page.
                    // Changes made to private mapping should not be persistent
                    Self::map_copy(addr.align_down(PAGE_SIZE), p.page().to_virt(), self.prot);

                    f.active_mappings.remove(&p.cache_key());
                } else {
                    return false;
                }

                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Clone)]
struct VMData {
    maps: LinkedList<Mapping>,
}

impl VMData {
    fn new() -> VMData {
        VMData {
            maps: LinkedList::new(),
        }
    }

    fn mmap_vm(
        &mut self,
        addr: Option<VirtAddr>,
        len: usize,
        prot: Prot,
        flags: Flags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Option<VirtAddr> {
        // support only fixed mappings for now
        if !flags.contains(Flags::MAP_FIXED) || addr.is_none() {
            return None;
        }

        // Offset should be multiple of PAGE_SIZE
        if offset % PAGE_SIZE != 0 {
            return None;
        }

        if let Some(a) = addr {
            // Address should be multiple of PAGE_SIZE if we request fixed mapping
            if flags.contains(Flags::MAP_FIXED) && a.0 % PAGE_SIZE != 0 {
                return None;
            }
        }

        if let Some(f) = &file {
            // Check whether file supports mmaped access
            if f.inode().as_cacheable().is_none() {
                return None;
            }
        }

        let mut cur = self.maps.cursor_front_mut();

        let addr = addr.unwrap();

        // Find slot for the new mapping
        while let Some(c) = cur.current() {
            if c.start > addr {
                break;
            } else {
                cur.move_next();
            }
        }

        let map = move |mut cur: CursorMut<Mapping>| {
            cur.insert_before(Mapping::new(addr, len, prot, flags, file, offset));
        };

        return if let Some(c) = cur.current() {
            if addr + len <= c.start {
                map(cur);

                Some(addr)
            } else {
                None
            }
        } else {
            map(cur);

            Some(addr)
        };
    }

    fn handle_pagefault(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if let Some(map) = self
            .maps
            .iter_mut()
            .find(|e| addr >= e.start && addr < e.end)
        {
            if map.prot.is_empty() {
                return false;
            }
            if reason.contains(PageFaultReason::WRITE) && !map.prot.contains(Prot::PROT_WRITE) {
                return false;
            }
            if reason.contains(PageFaultReason::I_FETCH) && !map.prot.contains(Prot::PROT_EXEC) {
                return false;
            }

            let is_private = map.flags.contains(Flags::MAP_PRIVATE);
            let is_anonymous = map.flags.contains(Flags::MAP_ANONYOMUS);

            //println!(
            //    "page fault {} p {} a {} r {:?}",
            //    addr, is_private, is_anonymous, reason
            //);

            return match (is_private, is_anonymous) {
                (false, _) => {
                    if is_anonymous {
                        panic!("Invalid mapping? shared and anonymous");
                    }

                    false
                }
                (true, false) => map.handle_pf_private_file(reason, addr),
                (true, true) => map.handle_pf_private_anon(reason, addr),
            };
        } else {
            false
        }
    }

    fn load_bin(&mut self, exe: DirEntryItem) -> Option<VirtAddr> {
        if let Some(elf_page) = exe.inode().as_cacheable().unwrap().get_mmap_page(0) {
            let hdr = unsafe { ElfHeader::load(elf_page.data()) };

            for p in hdr.programs().filter(|p| p.p_type == ProgramType::Load) {
                let virt_begin = VirtAddr(p.p_vaddr as usize).align_down(PAGE_SIZE);
                let virt_end =
                    VirtAddr(p.p_vaddr as usize + p.p_memsz as usize).align_up(PAGE_SIZE);

                let file_offset = p.p_offset.align(PAGE_SIZE as u64);

                self.mmap_vm(
                    Some(virt_begin),
                    virt_end.0 - virt_begin.0,
                    p.p_flags.into(),
                    Flags::MAP_PRIVATE | Flags::MAP_FIXED,
                    Some(exe.clone()),
                    file_offset as usize,
                )
                .expect("Failed to mmap");
            }

            return Some(VirtAddr(hdr.e_entry as usize));
        }

        None
    }

    fn print_vm(&self) {
        for e in self.maps.iter() {
            println!(
                "{} {}: {:?}, {:?} [ {} ]",
                e.start,
                e.end,
                e.prot,
                e.flags,
                if let Some(f) = &e.mmaped_file {
                    f.file.full_path()
                } else {
                    String::from("")
                }
            );
        }
    }

    fn fork(&mut self, vm: &VM) {
        let other = vm.data.lock();

        self.maps = other.maps.clone();
    }
}

pub struct VM {
    data: Spin<VMData>,
}

bitflags! {
    pub struct PageFaultReason: usize {
        const PRESENT = 0b1;
        const WRITE = 0b10;
        const USER = 0b100;
        const RESV_WRITE = 0b1000;
        const I_FETCH = 0b10000;
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            data: Spin::new(VMData::new()),
        }
    }

    pub fn fork(&self, vm: &VM) {
        self.data.lock().fork(vm);
    }

    pub fn mmap_vm(
        &self,
        addr: Option<VirtAddr>,
        len: usize,
        prot: Prot,
        flags: Flags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Option<VirtAddr> {
        let mut data = self.data.lock();

        data.mmap_vm(addr, len, prot, flags, file, offset)
    }

    pub fn handle_pagefault(&self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        self.data.lock().handle_pagefault(reason, addr)
    }

    pub fn load_bin(&self, exe: DirEntryItem) -> Option<VirtAddr> {
        self.data.lock().load_bin(exe)
    }

    pub fn clear(&self) {
        self.data.lock().maps.clear();
    }

    pub fn print_vm(&self) {
        self.data.lock().print_vm();
    }
}
