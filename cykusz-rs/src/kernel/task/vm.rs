use alloc::collections::linked_list::CursorMut;
use alloc::collections::LinkedList;
use alloc::string::String;
use core::ops::Range;

use syscall_defs::{MMapFlags, MMapProt};

use crate::arch::mm::{MMAP_USER_ADDR, PAGE_SIZE};
use crate::arch::raw::mm::UserAddr;
use crate::drivers::elf::types::{ProgramFlags, ProgramType};
use crate::drivers::elf::ElfHeader;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::pcache::PageItem;
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{
    allocate_order, map_flags, map_to_flags, unmap, update_flags, VirtAddr, MAX_USER_ADDR,
};
use crate::kernel::sync::Spin;
use crate::kernel::utils::types::Align;

impl From<MMapProt> for PageFlags {
    fn from(e: MMapProt) -> Self {
        let mut res = PageFlags::empty();

        if !e.contains(MMapProt::PROT_EXEC) {
            res.insert(PageFlags::NO_EXECUTE);
        }

        if e.contains(MMapProt::PROT_WRITE) {
            res.insert(PageFlags::WRITABLE);
        }

        res
    }
}

impl From<ProgramFlags> for MMapProt {
    fn from(f: ProgramFlags) -> Self {
        let mut prot = MMapProt::empty();

        if f.contains(ProgramFlags::READABLE) {
            prot.insert(MMapProt::PROT_READ);
        }

        if f.contains(ProgramFlags::WRITABLE) {
            prot.insert(MMapProt::PROT_WRITE);
        }

        if f.contains(ProgramFlags::EXECUTABLE) {
            prot.insert(MMapProt::PROT_EXEC);
        }

        prot
    }
}

#[derive(Clone)]
struct MMapedFile {
    file: DirEntryItem,
    starting_offset: usize,
    active_mappings: hashbrown::HashMap<VirtAddr, PageItem>,
}

impl MMapedFile {
    fn new(file: DirEntryItem, offset: usize) -> MMapedFile {
        MMapedFile {
            file,
            starting_offset: offset,
            active_mappings: hashbrown::HashMap::new(),
        }
    }

    fn unmap(&mut self, addr: VirtAddr) {
        let addr = addr.align_down(PAGE_SIZE);
        let uaddr: UserAddr = addr.into();

        if let Some(m) = self.active_mappings.get(&addr) {
            m.drop_user_addr(&uaddr);

            self.active_mappings.remove(&addr);
        }
    }

    fn split_from(&mut self, start: VirtAddr, end: VirtAddr, new_offset: usize) -> MMapedFile {
        let mut new = MMapedFile::new(self.file.clone(), new_offset);

        for a in (start..end).step_by(PAGE_SIZE) {
            if let Some(pg) = self.active_mappings.remove(&a) {
                new.active_mappings.insert(a, pg);
            }
        }

        new
    }
}

impl Drop for MMapedFile {
    fn drop(&mut self) {
        for (&addr, mapping) in self.active_mappings.iter() {
            let uaddr: UserAddr = addr.into();

            mapping.drop_user_addr(&uaddr);
        }
    }
}

enum UnmapResult {
    None,
    Full,
    Begin,
    Mid(Mapping),
    End,
}

#[derive(Clone)]
pub struct Mapping {
    mmaped_file: Option<MMapedFile>,
    prot: MMapProt,
    flags: MMapFlags,
    start: VirtAddr,
    end: VirtAddr,
}

impl Mapping {
    fn new(
        addr: VirtAddr,
        len: usize,
        prot: MMapProt,
        flags: MMapFlags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Mapping {
        Mapping {
            mmaped_file: file.map(|e| MMapedFile::new(e, offset)),
            prot,
            flags,
            start: addr,
            end: addr + len,
        }
    }

    fn new_split(
        addr: VirtAddr,
        len: usize,
        prot: MMapProt,
        flags: MMapFlags,
        file: Option<MMapedFile>,
    ) -> Mapping {
        Mapping {
            mmaped_file: file,
            prot,
            flags,
            start: addr,
            end: addr + len,
        }
    }

    fn map_copy(addr: VirtAddr, src: VirtAddr, prot: MMapProt) {
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

            let addr_aligned = addr.align_down(PAGE_SIZE);

            if let Some(p) = f.file.inode().as_cacheable().unwrap().get_mmap_page(offset) {
                if !reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    // Page is not present and we are reading from it, so map it readable
                    f.active_mappings.insert(addr_aligned, p.clone());

                    let mut flags: PageFlags = PageFlags::USER | self.prot.into();
                    flags.remove(PageFlags::WRITABLE);

                    map_to_flags(addr.align_down(PAGE_SIZE), p.page(), flags);
                } else if reason.contains(PageFaultReason::WRITE) {
                    // We are writing to private file mapping so copy the content of the page.
                    // Changes made to private mapping should not be persistent
                    Self::map_copy(addr.align_down(PAGE_SIZE), p.page().to_virt(), self.prot);

                    f.active_mappings.remove(&addr_aligned);
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

    fn handle_pf_shared_file(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if let Some(f) = self.mmaped_file.as_mut() {
            let offset = (addr - self.start).0 + f.starting_offset;

            if let Some(p) = f.file.inode().as_cacheable().unwrap().get_mmap_page(offset) {
                let is_present = reason.contains(PageFaultReason::PRESENT);
                let is_write = reason.contains(PageFaultReason::WRITE);

                if is_present && !is_write {
                    // We want to read present page, this page fault should not happen so return false..

                    return false;
                }

                let mut flags: PageFlags = PageFlags::from(self.prot) | PageFlags::USER;

                let addr_aligned = addr.align_down(PAGE_SIZE);

                if !is_present {
                    // Insert page to the list of active mappings if not present
                    f.active_mappings.insert(addr_aligned, p.clone());
                }

                if is_write {
                    // We want to write so make the page writable and send notify
                    map_to_flags(addr_aligned, p.page(), flags);

                    p.notify_dirty(&p, Some(addr_aligned.into()));
                } else {
                    // Page is not present and we are reading, so map it readable
                    flags.remove(PageFlags::WRITABLE);

                    map_to_flags(addr_aligned, p.page(), flags);
                }

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn split_from(&mut self, addr: VirtAddr) -> Mapping {
        assert!(addr > self.start && addr < self.end);
        let new_f = if let Some(f) = &mut self.mmaped_file {
            Some(f.split_from(addr, self.end, f.starting_offset + (addr - self.start).0))
        } else {
            None
        };

        Mapping::new_split(addr, (self.end - addr).0, self.prot, self.flags, new_f)
    }

    fn update_start(&mut self, new_start: VirtAddr) {
        assert!(new_start > self.start && new_start < self.end);

        let offset = (new_start - self.start).0;

        if let Some(f) = &mut self.mmaped_file {
            f.starting_offset += offset;
        }

        self.start = new_start;
    }

    fn update_end(&mut self, new_end: VirtAddr) {
        assert!(new_end > self.start && new_end < self.end);

        self.end = new_end;
    }

    fn unmap(&mut self, start: VirtAddr, end: VirtAddr) -> UnmapResult {
        assert_eq!(start.0 % PAGE_SIZE, 0);
        assert_eq!(end.0 % PAGE_SIZE, 0);

        let unmap_range = |range: Range<VirtAddr>, f: &mut Option<MMapedFile>| {
            for v in range.step_by(PAGE_SIZE) {
                if let Some(f) = f {
                    f.unmap(v);
                }

                unmap(v);
            }
        };

        //....>--<..############..>--<
        if end <= self.start || start >= self.end {
            return UnmapResult::None;
        }

        //..........###>----<###......
        if start > self.start && end < self.end {
            unmap_range(start..end, &mut self.mmaped_file);

            let split = self.split_from(end);
            //MID
            self.update_end(start);

            return UnmapResult::Mid(split);
        }

        //..........>----------<.....
        if start <= self.start && end >= self.end {
            //FULL
            unmap_range(self.start..self.end, &mut self.mmaped_file);

            return UnmapResult::Full;
        }

        //..........>--------<###......
        if start <= self.start && end < self.end {
            //BEGIN
            unmap_range(self.start..end, &mut self.mmaped_file);

            self.update_start(end);

            return UnmapResult::Begin;
        }

        //..........###>--------<......
        if start > self.start && end >= self.end {
            unmap_range(start..self.end, &mut self.mmaped_file);

            self.update_end(start);

            return UnmapResult::End;
            //END
        }

        unreachable!()
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

    fn find_fixed(&mut self, addr: VirtAddr, len: usize) -> Option<(VirtAddr, CursorMut<Mapping>)> {
        let mut cur = self.maps.cursor_front_mut();

        while let Some(c) = cur.current() {
            if c.start <= addr && c.end > addr {
                return None;
            } else if c.start < addr {
                cur.move_next();
            } else {
                if addr + len > c.start {
                    return None;
                } else {
                    break;
                }
            }
        }

        return Some((addr, cur));
    }

    fn find_any_above(
        &mut self,
        _addr: VirtAddr,
        _len: usize,
    ) -> Option<(VirtAddr, CursorMut<Mapping>)> {
        None
    }

    fn mmap_vm(
        &mut self,
        addr: Option<VirtAddr>,
        len: usize,
        prot: MMapProt,
        flags: MMapFlags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Option<VirtAddr> {
        // Offset should be multiple of PAGE_SIZE
        if offset % PAGE_SIZE != 0 {
            return None;
        }

        if len == 0 {
            return None;
        }

        let len = len.align_up(PAGE_SIZE);

        if let Some(a) = addr {
            // Address should be multiple of PAGE_SIZE if we request fixed mapping
            // and should not extend beyond max user addr
            if flags.contains(MMapFlags::MAP_FIXED) && (a.0 % PAGE_SIZE != 0 || a + len > MAX_USER_ADDR)
            {
                return None;
            }
        }

        if let Some(f) = &file {
            // Can't mmap file with anonymous flag
            if flags.contains(MMapFlags::MAP_ANONYOMUS) {
                return None;
            }

            // Check whether file supports mmaped access
            if f.inode().as_cacheable().is_none() {
                return None;
            }
        } else {
            // Mappings not backed by the file must be anonymous
            if !flags.contains(MMapFlags::MAP_ANONYOMUS) {
                return None;
            }

            // Can't have shared anonymous mapping
            if flags.contains(MMapFlags::MAP_SHARED) {
                return None;
            }
        }

        match addr {
            Some(addr) => {
                if flags.contains(MMapFlags::MAP_FIXED) {
                    // Remove any existing mappings
                    self.unmap(addr, len);

                    if let Some(c) = self.find_fixed(addr, len) {
                        Some(c)
                    } else {
                        None
                    }
                } else {
                    self.find_any_above(addr, len)
                }
            }
            None => self.find_any_above(MMAP_USER_ADDR, len),
        }
        .and_then(|(addr, mut cur)| {
            cur.insert_before(Mapping::new(addr, len, prot, flags, file, offset));

            Some(addr)
        })
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
            if reason.contains(PageFaultReason::WRITE) && !map.prot.contains(MMapProt::PROT_WRITE) {
                return false;
            }
            if reason.contains(PageFaultReason::I_FETCH) && !map.prot.contains(MMapProt::PROT_EXEC)
            {
                return false;
            }

            let is_private = map.flags.contains(MMapFlags::MAP_PRIVATE);
            let is_anonymous = map.flags.contains(MMapFlags::MAP_ANONYOMUS);

            println!(
                "page fault {} p {} a {} r {:?}",
                addr, is_private, is_anonymous, reason
            );

            return match (is_private, is_anonymous) {
                (false, _) => {
                    if is_anonymous {
                        panic!("Invalid mapping? shared and anonymous");
                    }

                    map.handle_pf_shared_file(reason, addr)
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
                    MMapFlags::MAP_PRIVATE | MMapFlags::MAP_FIXED,
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

    fn unmap(&mut self, addr: VirtAddr, len: usize) -> bool {
        let start = addr.align_up(PAGE_SIZE);
        let end = (addr + len).align_up(PAGE_SIZE);

        let mut cursor = self.maps.cursor_front_mut();

        let mut success = false;

        while let Some(c) = cursor.current() {
            if c.end < start {
                cursor.move_next();
            } else {
                match c.unmap(start, end) {
                    UnmapResult::None => {
                        return success;
                    }
                    UnmapResult::Full => {
                        success = true;

                        cursor.remove_current();
                    }
                    UnmapResult::Begin => {
                        return true;
                    }
                    UnmapResult::End => {
                        success = true;

                        cursor.move_next();
                    }
                    UnmapResult::Mid(new_mapping) => {
                        cursor.insert_after(new_mapping);

                        return true;
                    }
                }
            }
        }

        success
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
        prot: MMapProt,
        flags: MMapFlags,
        file: Option<DirEntryItem>,
        offset: usize,
    ) -> Option<VirtAddr> {
        let mut data = self.data.lock();

        data.mmap_vm(addr, len, prot, flags, file, offset)
    }

    pub fn munmap_vm(&self, addr: VirtAddr, len: usize) -> bool {
        let mut data = self.data.lock();

        data.unmap(addr, len)
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
