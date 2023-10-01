use alloc::collections::linked_list::CursorMut;
use alloc::collections::LinkedList;

use core::ops::Range;

use syscall_defs::{MMapFlags, MMapProt};

use crate::arch::mm::{MMAP_USER_ADDR, PAGE_SIZE};
use crate::arch::raw::mm::UserAddr;
use crate::drivers::elf::types::{BinType, ProgramFlags, ProgramType};
use crate::drivers::elf::ElfHeader;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::pcache::{MMapPage, MMapPageStruct, PageCacheItemArc};
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{
    allocate_order, map_flags, map_to_flags, unmap, update_flags, VirtAddr, MAX_USER_ADDR,
};
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::Mutex;
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
    len: usize,
    active_mappings: hashbrown::HashMap<VirtAddr, PageCacheItemArc>,
}

impl MMapedFile {
    fn new(file: DirEntryItem, len: usize, offset: usize) -> MMapedFile {
        MMapedFile {
            file,
            starting_offset: offset,
            len,
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
        assert!(self.len > new_offset - self.starting_offset);

        let new_len = self.len - (new_offset - self.starting_offset);

        let mut new = MMapedFile::new(self.file.clone(), new_len, new_offset);

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
            mmaped_file: file.map(|e| MMapedFile::new(e, len, offset)),
            prot,
            flags,
            start: addr,
            end: addr + len.align_up(PAGE_SIZE),
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

    fn map_copy(addr: VirtAddr, src: VirtAddr, bytes: usize, prot: MMapProt) {
        let mut new_page = allocate_order(0).unwrap();
        new_page.clear();

        unsafe {
            new_page
                .address_mapped()
                .as_virt()
                .copy_page_from_bytes(src, bytes);
        }

        map_to_flags(addr, new_page.address(), PageFlags::USER | prot.into());
    }

    fn handle_pf_private_anon(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        let addr_aligned = addr.align_down(PAGE_SIZE);

        if !reason.contains(PageFaultReason::PRESENT) {
            // Page not present so just make it available
            logln_disabled!("map: private read");
            map_flags(addr_aligned, PageFlags::USER | self.prot.into());

            true
        } else if reason.contains(PageFaultReason::WRITE) {
            logln_disabled!("map: handle cow anon");
            return self.handle_cow(addr_aligned, false, PAGE_SIZE);
        } else {
            logln_disabled!("map: present read fail");
            false
        }
    }

    fn handle_pf_private_file(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if let Some(f) = self.mmaped_file.as_mut() {
            let offset = (addr - self.start).0 + f.starting_offset;

            let addr_aligned = addr.align_down(PAGE_SIZE);

            let addr_offset = (addr_aligned - self.start).0;
            let bytes = core::cmp::min(PAGE_SIZE, f.len - addr_offset);

            if current_task_ref().locks() > 0 {
                logln!("handle_pf_private_file: locks > 0");
            }

            let mappable = f.file.inode().as_mappable().unwrap();

            if let Some(MMapPageStruct(MMapPage::Cached(p))) = mappable.get_mmap_page(offset) {
                if !reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    if bytes == PAGE_SIZE {
                        // Page is not present and we are reading from it, so map it readable
                        f.active_mappings.insert(addr_aligned, p.clone());

                        logln_disabled!("map read {}", addr_aligned);

                        let mut flags: PageFlags = PageFlags::USER | self.prot.into();
                        flags.remove(PageFlags::WRITABLE);

                        map_to_flags(addr_aligned, p.page(), flags);
                    } else {
                        logln_disabled!("map read copy {} {}", addr_aligned, bytes);

                        Self::map_copy(addr_aligned, p.page().to_virt(), bytes, self.prot);

                        f.active_mappings.remove(&addr_aligned);
                    }
                } else if reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    // We are writing to private file mapping so copy the content of the page.
                    // Changes made to private mapping should not be persistent

                    logln_disabled!("map copy {} {}", addr_aligned, bytes);

                    Self::map_copy(addr_aligned, p.page().to_virt(), bytes, self.prot);

                    f.active_mappings.remove(&addr_aligned);
                } else if reason.contains(PageFaultReason::PRESENT)
                    && reason.contains(PageFaultReason::WRITE)
                {
                    logln_disabled!("map: handle cow priv file");

                    return if self.handle_cow(addr_aligned, true, PAGE_SIZE) {
                        self.mmaped_file
                            .as_mut()
                            .expect("unreachable")
                            .active_mappings
                            .remove(&addr_aligned);

                        true
                    } else {
                        false
                    };
                }

                true
            } else {
                //println!("failed to get mmap page");
                //map_flags(addr.align_down(PAGE_SIZE), PageFlags::USER | self.prot.into());
                false
            }
        } else {
            false
        }
    }

    fn handle_cow(&mut self, addr_aligned: VirtAddr, do_copy: bool, bytes: usize) -> bool {
        if let Some(phys) = addr_aligned.to_phys_pagewalk() {
            if let Some(phys_page) = phys.to_phys_page() {
                // If there is more than one process mapping this page, make a private copy
                // Otherwise, this page is not shared with anyone, so just make it writable
                if phys_page.vm_use_count() > 1 || do_copy {
                    logln_disabled!("mmap cow: map_copy {}", bytes);
                    Self::map_copy(addr_aligned, addr_aligned, bytes, self.prot);
                } else {
                    logln_disabled!("mmap cow: update flags");
                    if !update_flags(addr_aligned, PageFlags::USER | self.prot.into()) {
                        panic!("update flags failed");
                    }
                };

                return true;
            }
        }

        false
    }

    fn handle_pf_shared_file(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if let Some(f) = self.mmaped_file.as_mut() {
            let offset = (addr - self.start).0 + f.starting_offset;

            if current_task_ref().locks() > 0 {
                logln!("handle_pf_shared: locks > 0");
            }
            let is_present = reason.contains(PageFaultReason::PRESENT);
            let is_write = reason.contains(PageFaultReason::WRITE);

            if is_present && !is_write {
                // We want to read present page, this page fault should not happen so return false..
                return false;
            }

            let mut flags: PageFlags = PageFlags::from(self.prot) | PageFlags::USER;

            let addr_aligned = addr.align_down(PAGE_SIZE);

            let mappable = f.file.inode().as_mappable().unwrap();

            match mappable.get_mmap_page(offset) {
                Some(MMapPageStruct(MMapPage::Cached(p))) => {
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
                }
                Some(MMapPageStruct(MMapPage::Direct(p))) => {
                    map_to_flags(addr_aligned, p.page(), flags | p.flags());
                    true
                }
                _ => false,
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

pub struct TlsVmInfo {
    pub file: DirEntryItem,
    pub file_offset: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub mmap_addr_hint: VirtAddr,
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
        addr: VirtAddr,
        len: usize,
    ) -> Option<(VirtAddr, CursorMut<Mapping>)> {
        use core::cmp::max;

        if self.maps.is_empty() {
            return Some((addr, self.maps.cursor_front_mut()));
        }

        let mut cur = self.maps.cursor_front_mut();

        while let Some(c) = cur.current() {
            let c_start = c.start;

            if c_start < addr {
                cur.move_next();
            } else {
                if let Some(p) = cur.peek_prev() {
                    let start = max(addr, p.end);
                    let hole = c_start.0 - start.0;

                    if len <= hole {
                        return Some((start, cur));
                    } else {
                        cur.move_next();
                    }
                } else {
                    let start = addr;

                    let hole = c_start.0 - addr.0;

                    return if len <= hole {
                        Some((start, cur))
                    } else {
                        None
                    };
                }
            }
        }

        if let Some(p) = cur.peek_prev() {
            let start = max(p.end, addr);

            let hole = MAX_USER_ADDR.0 - start.0;

            if hole >= len {
                return Some((start, cur));
            }
        }

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

        if let Some(a) = addr {
            // Address should be multiple of PAGE_SIZE if we request fixed mapping
            // and should not extend beyond max user addr
            if flags.contains(MMapFlags::MAP_FIXED)
                && (a.0 % PAGE_SIZE != 0 || a + len > MAX_USER_ADDR)
            {
                return None;
            }

            if !a.is_user() {
                return None;
            }
        }

        if let Some(f) = &file {
            // Can't mmap file with anonymous flag
            if flags.contains(MMapFlags::MAP_ANONYOMUS) {
                return None;
            }

            // Check whether file supports mmaped access
            if f.inode().as_mappable().is_none() {
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
                    self.unmap(addr, len.align_up(PAGE_SIZE));

                    if let Some(c) = self.find_fixed(addr, len.align_up(PAGE_SIZE)) {
                        Some(c)
                    } else {
                        None
                    }
                } else {
                    self.find_any_above(addr, len)
                }
            }
            None => self.find_any_above(MMAP_USER_ADDR, len.align_up(PAGE_SIZE)),
        }
        .and_then(|(addr, mut cur)| {
            if let Some(prev) = cur.peek_prev() {
                if prev.end == addr
                    && prev.flags == flags
                    && prev.prot == prot
                    && prev.mmaped_file.is_none()
                {
                    prev.end = addr + len.align_up(PAGE_SIZE);

                    return Some(addr);
                }
            }

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

            logln_disabled!(
                "page fault {} p {} a {} {:?} pid {}",
                addr.align_down(PAGE_SIZE),
                is_private,
                is_anonymous,
                reason,
                current_task_ref().tid(),
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
            logln_disabled!("task {}: mmap not found", current_task_ref().tid());
            //self.print_vm();
            false
        }
    }

    fn load_bin(
        &mut self,
        exe: DirEntryItem,
    ) -> Option<(VirtAddr, VirtAddr, ElfHeader, Option<TlsVmInfo>)> {
        let mut base_addr = VirtAddr(0);

        if let Some(MMapPageStruct(MMapPage::Cached(elf_page))) =
            exe.inode().as_mappable()?.get_mmap_page(0)
        {
            let hdr = unsafe { ElfHeader::load(elf_page.data()) };

            if hdr.is_none()
            /*|| exe.full_path().ends_with("cc1")*/
            {
                println!("failed elf {:?}", &elf_page.data()[..256]);
                return None;
            }

            let hdr = hdr.unwrap();

            let load_offset = VirtAddr(if hdr.e_type == BinType::Dyn {
                0x7500_0000_0000usize
            } else {
                0usize
            });

            let mut entry_addr = VirtAddr(hdr.e_entry as usize) + load_offset;

            let mut last_mmap_end = VirtAddr(0);
            let mut tls_vm_info = None;

            for p in hdr
                .programs()
                .filter(|p| p.p_type == ProgramType::Load || /*p.p_type == ProgramType::TLS || */p.p_type == ProgramType::Interp)
            {
                if p.p_type == ProgramType::Interp {
                    if let Ok(interp) = lookup_by_path(Path::new("/usr/lib/ld.so"), LookupMode::None) {
                        if let Some((_base_addr, entry, _elf, _tls)) = self.load_bin(interp) {
                            entry_addr = entry;
                        }
                    } else {
                        return None;
                    }

                } else if p.p_type == ProgramType::Load {
                    let virt_begin = load_offset + VirtAddr(p.p_vaddr as usize).align_down(PAGE_SIZE);

                    if base_addr == VirtAddr(0) {
                        base_addr = virt_begin;
                    }

                    let virt_fend = load_offset + VirtAddr(p.p_vaddr as usize + p.p_filesz as usize);

                    let virt_end =
                        load_offset + VirtAddr(p.p_vaddr as usize + p.p_memsz as usize).align_up(PAGE_SIZE);

                    let len = (virt_fend - virt_begin).0;

                    let file_offset = p.p_offset.align(PAGE_SIZE as u64);

                    logln!(
                        "mmap {} - {} offset {:#x}",
                        virt_begin, virt_fend, file_offset
                    );

                    last_mmap_end = self
                        .mmap_vm(
                            Some(virt_begin),
                            len,
                            p.p_flags.into(),
                            MMapFlags::MAP_PRIVATE | MMapFlags::MAP_FIXED,
                            Some(exe.clone()),
                            file_offset as usize,
                        )
                        .expect("Failed to mmap")
                        + len.align_up(PAGE_SIZE);

                    let virt_fend = last_mmap_end;

                    if virt_fend < virt_end {
                        //println!("mmap {} - {} offset {:#x}", virt_fend, virt_end, 0);
                        let len = (virt_end - virt_fend).0;

                        last_mmap_end = self
                            .mmap_vm(
                                Some(virt_fend),
                                virt_end.0 - virt_fend.0,
                                p.p_flags.into(),
                                MMapFlags::MAP_PRIVATE
                                    | MMapFlags::MAP_ANONYOMUS
                                    | MMapFlags::MAP_FIXED,
                                None,
                                0,
                            )
                            .expect("Failed to mmap")
                            + len;
                    }
                } else {
                    if tls_vm_info.is_some() {
                        panic!("TLS already setup");
                    }
                    let tls_mem_size = p.p_memsz as usize;
                    let tls_file_size = p.p_filesz as usize;
                    let tls_file_offset = p.p_offset as usize;

                    tls_vm_info = Some(TlsVmInfo {
                        file: exe.clone(),
                        file_offset: tls_file_offset,
                        file_size: tls_file_size,
                        mem_size: tls_mem_size,
                        mmap_addr_hint: last_mmap_end,
                    });
                }
            }

            return Some((base_addr, entry_addr, *hdr, tls_vm_info));
        }

        None
    }

    fn print_vm(&self) {
        for e in self.maps.iter() {
            if let Some(f) = &e.mmaped_file {
                println!(
                    "{} {}: {:?}, {:?} [ {} {:#x} {:#x} ]",
                    e.start,
                    e.end,
                    e.prot,
                    e.flags,
                    f.file.full_path(),
                    f.starting_offset,
                    f.len,
                );
            } else {
                println!("{} {}: {:?}, {:?}", e.start, e.end, e.prot, e.flags,);
            }
        }
    }

    fn log_vm(&self) {
        for e in self.maps.iter() {
            if let Some(f) = &e.mmaped_file {
                logln2!(
                    "{} {}: {:?}, {:?} [ {} {:#x} {:#x} ]",
                    e.start,
                    e.end,
                    e.prot,
                    e.flags,
                    f.file.full_path(),
                    f.starting_offset,
                    f.len,
                );
            } else {
                logln2!("{} {}: {:?}, {:?}", e.start, e.end, e.prot, e.flags,);
            }
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
            if c.end <= start {
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
    data: Mutex<VMData>,
}

impl Default for VM {
    fn default() -> VM {
        VM::new()
    }
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
            data: Mutex::new(VMData::new()),
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
        let mut res = self.data.lock();

        let ret = res.handle_pagefault(reason, addr);

        ret
    }

    pub fn load_bin(
        &self,
        exe: DirEntryItem,
    ) -> Option<(VirtAddr, VirtAddr, ElfHeader, Option<TlsVmInfo>)> {
        self.data.lock().load_bin(exe)
    }

    pub fn clear(&self) {
        self.data.lock().maps.clear();
    }

    pub fn print_vm(&self) {
        self.data.lock().print_vm();
    }

    pub fn log_vm(&self) {
        self.data.lock().log_vm();
    }
}
