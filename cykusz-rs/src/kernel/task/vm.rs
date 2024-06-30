use alloc::boxed::Box;
use alloc::collections::linked_list::CursorMut;
use alloc::collections::LinkedList;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::ops::Range;
use syscall_defs::exec::ExeArgs;
use syscall_defs::{MMapFlags, MMapProt, OpenFlags, SyscallError, SyscallResult};

use crate::arch::mm::{MMAP_USER_ADDR, PAGE_SIZE};
use crate::arch::raw::mm::UserAddr;
use crate::drivers::elf::types::{BinType, ProgramFlags, ProgramType};
use crate::drivers::elf::ElfHeader;
use crate::kernel::fs::dirent::{DirEntry, DirEntryItem};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::pcache::{
    MMapPage, MMapPageStruct, MappedAccess, PageCacheItemArc, PageDirectItemStruct,
};
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{
    allocate_order, map_flags, map_to_flags, unmap, update_flags, PhysAddr, VirtAddr, MAX_USER_ADDR,
};
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::{LockApi, Mutex};
use crate::kernel::task::filetable::FileHandle;
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

struct AnonymousSharedMem {
    pages: Mutex<hashbrown::HashMap<usize, PhysAddr>>,
    len: usize,
    flags: PageFlags,
    sref: Weak<AnonymousSharedMem>,
}

impl Drop for AnonymousSharedMem {
    fn drop(&mut self) {
        dbgln!(map_call, "Drop AnonymousSharedMem");
    }
}

impl AnonymousSharedMem {
    fn get_file_handle(len: usize, flags: PageFlags) -> Arc<FileHandle> {
        let inode = Arc::<AnonymousSharedMem>::new_cyclic(|me| AnonymousSharedMem {
            pages: Mutex::new(hashbrown::HashMap::new()),
            len,
            flags,
            sref: me.clone(),
        });

        FileHandle::new(0, DirEntry::inode_wrap(inode), OpenFlags::RDWR, 0)
    }
}

impl MappedAccess for AnonymousSharedMem {
    fn get_mmap_page(&self, offset: usize, size_check: bool) -> Option<MMapPageStruct> {
        let mut pages = self.pages.lock();

        let offset = offset.align_down(PAGE_SIZE);

        if offset >= self.len && size_check {
            return None;
        }

        let phys_page = if pages.contains_key(&offset) {
            let p = pages.get(&offset).unwrap();

            *p
        } else {
            let mut page = allocate_order(0).expect("Failed to alloc page");
            page.clear();

            pages.insert(offset, page.address());

            page.address()
        };

        dbgln!(
            vm,
            "AnonSharedMem got page {} offset {} flags {:?}",
            phys_page,
            offset,
            self.flags
        );

        Some(MMapPageStruct(MMapPage::Direct(PageDirectItemStruct::new(
            phys_page, offset, self.flags,
        ))))
    }
}

impl INode for AnonymousSharedMem {
    fn as_mappable(&self) -> Option<Arc<dyn MappedAccess>> {
        Some(self.sref.upgrade().unwrap())
    }
}

#[derive(Clone)]
struct MMapedFile {
    file: Arc<FileHandle>,
    starting_offset: usize,
    len: usize,
    active_mappings: hashbrown::HashMap<VirtAddr, PageCacheItemArc>,
}

impl MMapedFile {
    fn new(file: Arc<FileHandle>, len: usize, offset: usize) -> MMapedFile {
        dbgln!(map_call, "mapped file: {}", Arc::strong_count(&file));
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
        self.len -= new_len;

        let mut new = MMapedFile::new(self.file.clone(), new_len, new_offset);

        for a in (start..end).step_by(PAGE_SIZE) {
            if let Some(pg) = self.active_mappings.remove(&a) {
                new.active_mappings.insert(a, pg);
            }
        }

        new
    }

    fn file(&self) -> &Arc<FileHandle> {
        &self.file
    }

    fn merge(&mut self, other: &mut MMapedFile, start: VirtAddr, end: VirtAddr) {
        assert!(Arc::ptr_eq(&self.file, &other.file));

        self.starting_offset = core::cmp::min(self.starting_offset, other.starting_offset);
        self.len += other.len;

        for a in (start..end).step_by(PAGE_SIZE) {
            if let Some(pg) = other.active_mappings.remove(&a) {
                self.active_mappings.insert(a, pg);
            }
        }
    }
}

impl Drop for MMapedFile {
    fn drop(&mut self) {
        dbgln!(
            map_call,
            "drop mapped file rc {} {}",
            Arc::strong_count(&self.file),
            self.file.get_fs_dir_item().full_path()
        );
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

enum MProtectResult {
    None,
    Full,
    Begin(Mapping),
    Mid(Mapping, Mapping),
    End(Mapping),
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
        file: Option<Arc<FileHandle>>,
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

    fn try_merge(&mut self, other: &mut Mapping) -> bool {
        if let (Some(mf), Some(omf)) = (&self.mmaped_file, &other.mmaped_file) {
            if !Arc::ptr_eq(mf.file(), omf.file()) {
                return false;
            } else if (mf.starting_offset + mf.len != omf.starting_offset)
                && (omf.starting_offset + omf.len != mf.starting_offset)
            {
                return false;
            }
        }

        if self.mmaped_file.is_none() != other.mmaped_file.is_none()
            || self.flags != other.flags
            || self.prot != other.prot
        {
            return false;
        }

        let res = if self.start == other.end {
            self.start = other.start;

            true
        } else if self.end == other.start {
            self.end = other.end;

            true
        } else {
            false
        };

        if res && self.mmaped_file.is_some() {
            self.mmaped_file.as_mut().unwrap().merge(
                other.mmaped_file.as_mut().unwrap(),
                other.start,
                other.end,
            );
        }

        res
    }

    fn handle_pf_private_anon(&mut self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        let addr_aligned = addr.align_down(PAGE_SIZE);

        if !reason.contains(PageFaultReason::PRESENT) {
            // Page not present so just make it available
            dbgln!(vm, "private read");
            map_flags(addr_aligned, PageFlags::USER | self.prot.into());

            true
        } else if reason.contains(PageFaultReason::WRITE) {
            dbgln!(vm, "handle cow anon");
            return self.handle_cow(addr_aligned, false, PAGE_SIZE);
        } else {
            dbgln!(vm, "present read fail");
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
                dbgln!(
                    vm,
                    "handle_pf_private_file: locks > 0 f: {}",
                    f.file.get_fs_dir_item().full_path()
                );
            }

            let mappable = f.file.get_dir_item().inode().as_mappable().unwrap();

            if let Some(MMapPageStruct(MMapPage::Cached(p))) = mappable.get_mmap_page(offset, false)
            {
                if !reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    if bytes == PAGE_SIZE {
                        // Page is not present and we are reading from it, so map it readable
                        f.active_mappings.insert(addr_aligned, p.clone());

                        dbgln!(vm, "map read {}", addr_aligned);

                        let mut flags: PageFlags = PageFlags::USER | self.prot.into();
                        flags.remove(PageFlags::WRITABLE);

                        map_to_flags(addr_aligned, p.page(), flags);
                    } else {
                        dbgln!(vm, "map read copy {} {}", addr_aligned, bytes);

                        Self::map_copy(addr_aligned, p.page().to_virt(), bytes, self.prot);

                        f.active_mappings.remove(&addr_aligned);
                    }
                } else if reason.contains(PageFaultReason::WRITE)
                    && !reason.contains(PageFaultReason::PRESENT)
                {
                    // We are writing to private file mapping so copy the content of the page.
                    // Changes made to private mapping should not be persistent

                    dbgln!(vm, "map copy {} {}", addr_aligned, bytes);

                    Self::map_copy(addr_aligned, p.page().to_virt(), bytes, self.prot);

                    f.active_mappings.remove(&addr_aligned);
                } else if reason.contains(PageFaultReason::PRESENT)
                    && reason.contains(PageFaultReason::WRITE)
                {
                    dbgln!(vm, "map: handle cow priv file");

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

            let mappable = f.file.get_dir_item().inode().as_mappable().unwrap();

            match mappable.get_mmap_page(offset, false) {
                Some(MMapPageStruct(MMapPage::Cached(p))) => {
                    if !is_present {
                        // Insert page to the list of active mappings if not present
                        f.active_mappings.insert(addr_aligned, p.clone());
                    }

                    if is_write {
                        // We want to write so make the page writable and send notify
                        map_to_flags(addr_aligned, p.page(), flags);

                        dbgln!(vm, "map writable");

                        p.notify_dirty(&p, Some(addr_aligned.into()));
                    } else {
                        // Page is not present and we are reading, so map it readable
                        flags.remove(PageFlags::WRITABLE);

                        dbgln!(vm, "map readonly");

                        map_to_flags(addr_aligned, p.page(), flags);
                    }

                    true
                }
                Some(MMapPageStruct(MMapPage::Direct(p))) => {
                    dbgln!(vm, "map direct");
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
        dbgln!(map_v, "split_from {} [{} {}]", addr, self.start, self.end);
        assert!(addr > self.start && addr < self.end);
        let new_f = if let Some(f) = &mut self.mmaped_file {
            Some(f.split_from(addr, self.end, f.starting_offset + (addr - self.start).0))
        } else {
            None
        };

        let new_split = Mapping::new_split(addr, (self.end - addr).0, self.prot, self.flags, new_f);
        dbgln!(map_v, "self [{} {}]", self.start, addr);
        dbgln!(map_v, "new  [{} {}]", addr, addr + (self.end - addr));

        self.update_end(addr);

        new_split
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
        assert_eq!(start.0.is_multiple_of(PAGE_SIZE), true);
        assert_eq!(end.0.is_multiple_of(PAGE_SIZE), true);

        dbgln!(unmap, "unmapping region {} - {}", self.start, self.end);

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
            dbgln!(unmap, "MID");
            unmap_range(start..end, &mut self.mmaped_file);

            //let _split1 = self.split_from(start);
            let split2 = self.split_from(start).split_from(end);
            //MID

            return UnmapResult::Mid(split2);
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

    fn mprotect(
        &mut self,
        start: VirtAddr,
        end: VirtAddr,
        prot: MMapProt,
    ) -> Result<MProtectResult, SyscallError> {
        assert!(start.0.is_multiple_of(PAGE_SIZE));
        assert!(end.0.is_multiple_of(PAGE_SIZE));

        if let Some(f) = &self.mmaped_file {
            if self.flags.contains(MMapFlags::MAP_SHARED)
                && prot.contains(MMapProt::PROT_WRITE)
                && !f.file.flags().is_writable()
            {
                return Err(SyscallError::EACCES);
            }
        }

        if self.prot == prot {
            dbgln!(map_call, "mprotect full 1");
            return Ok(MProtectResult::Full);
        }

        //....>--<..############..>--<
        if end <= self.start || start >= self.end {
            return Ok(MProtectResult::None);
        }

        //..........###>----<###......
        if start > self.start && end < self.end {
            let mut split1 = self.split_from(start);
            let split2 = split1.split_from(end);

            split1.prot = prot;
            //MID

            return Ok(MProtectResult::Mid(split1, split2));
        }

        //..........>----------<.....
        if start <= self.start && end >= self.end {
            //FULL
            self.prot = prot;

            return Ok(MProtectResult::Full);
        }

        //..........>--------<###......
        if start <= self.start && end < self.end {
            //BEGIN
            self.prot = prot;
            let split = self.split_from(end);

            return Ok(MProtectResult::Begin(split));
        }

        //..........###>--------<......
        if start > self.start && end >= self.end {
            let mut split = self.split_from(start);
            split.prot = prot;

            return Ok(MProtectResult::End(split));
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

trait InsertMergeAfter {
    fn insert_merge_after<'a>(self, mapping: Mapping) -> Self;
    fn insert_merge_before<'a>(self, mapping: Mapping) -> Self;

    fn merge_prev_next<'a>(self) -> Self;
}

impl<'a> InsertMergeAfter for CursorMut<'a, Mapping> {
    fn insert_merge_after(mut self, mut mapping: Mapping) -> Self {
        if if let Some(current) = self.current() {
            mapping.try_merge(current)
        } else {
            false
        } {
            // merged
            self.remove_current();
        } else {
            self.move_next();
        }

        if let Some(next) = self.current() {
            if !next.try_merge(&mut mapping) {
                self.insert_before(mapping);
                self.move_prev();
            }
        } else {
            self.insert_before(mapping);
            self.move_prev();
        }

        self
    }

    fn insert_merge_before(mut self, mapping: Mapping) -> Self {
        self.move_prev();

        self.insert_merge_after(mapping)
    }

    fn merge_prev_next(mut self) -> Self {
        dbgln!(map_call, "merge prev next");
        let current = self.remove_current();

        if let Some(mut current) = current {
            if if let Some(prev) = self.peek_prev() {
                current.try_merge(prev)
            } else {
                false
            } {
                self.move_prev();
                self.insert_after(current);
                self.remove_current();
            } else {
                self.insert_before(current);
                self.move_prev();
            }
        }

        let current = self.remove_current();

        if let Some(mut current) = current {
            if if let Some(next) = self.current() {
                current.try_merge(next)
            } else {
                false
            } {
                self.remove_current();
                self.move_prev();
                self.insert_after(current);
            } else {
                self.insert_before(current);
                self.move_prev();
            }
        }

        dbgln!(map_call, "merge prev next fin");

        self
    }
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

        Some((addr, cur))
    }

    fn find_any_above(
        &mut self,
        addr: VirtAddr,
        len: usize,
    ) -> Option<(VirtAddr, CursorMut<Mapping>)> {
        use core::cmp::max;

        dbgln!(map_v, "find_any_above {} {}", addr, len);

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

                    if len <= hole {
                        return Some((start, cur));
                    } else {
                        cur.move_next();
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

    fn mmap(
        &mut self,
        addr: Option<VirtAddr>,
        len: usize,
        prot: MMapProt,
        flags: MMapFlags,
        mut file: Option<Arc<FileHandle>>,
        offset: usize,
    ) -> Option<VirtAddr> {
        // Offset should be multiple of PAGE_SIZE
        if !offset.is_multiple_of(PAGE_SIZE) {
            return None;
        }

        if len == 0 {
            return None;
        }

        if let Some(a) = addr {
            // Address should be multiple of PAGE_SIZE if we request fixed mapping
            // and should not extend beyond max user addr
            if flags.contains(MMapFlags::MAP_FIXED)
                && (!a.0.is_multiple_of(PAGE_SIZE) || a + len > MAX_USER_ADDR)
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

            if !flags.intersects(MMapFlags::MAP_SHARED | MMapFlags::MAP_PRIVATE) {
                return None;
            }

            // Check whether file supports mmaped access
            if f.get_dir_item().inode().as_mappable().is_none() {
                return None;
            }
        } else {
            // Mappings not backed by the file must be anonymous
            if !flags.contains(MMapFlags::MAP_ANONYOMUS) {
                return None;
            }
        }

        if flags.contains(MMapFlags::MAP_ANONYOMUS | MMapFlags::MAP_SHARED) {
            dbgln!(map_call, "Alloc anon file");
            file = Some(AnonymousSharedMem::get_file_handle(
                len,
                PageFlags::USER | prot.into(),
            ));
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
        .and_then(|(addr, cur)| {
            let mapping = Mapping::new(addr, len, prot, flags, file, offset);
            cur.insert_merge_before(mapping);

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

            dbgln!(
                vm,
                "page fault {} p {} a {} {:?} pid {}",
                addr.align_down(PAGE_SIZE),
                is_private,
                is_anonymous,
                reason,
                current_task_ref().tid(),
            );

            match (is_private, is_anonymous) {
                (false, _) => map.handle_pf_shared_file(reason, addr),
                (true, false) => map.handle_pf_private_file(reason, addr),
                (true, true) => map.handle_pf_private_anon(reason, addr),
            }
        } else {
            logln_disabled!("task {}: mmap not found", current_task_ref().tid());
            //self.print_vm();
            false
        }
    }

    fn load_bin(
        &mut self,
        exe: DirEntryItem,
    ) -> Option<(
        VirtAddr,
        VirtAddr,
        ElfHeader,
        Option<TlsVmInfo>,
        Option<(DirEntryItem, Option<ExeArgs>)>,
    )> {
        let mut base_addr = VirtAddr(0);

        if let Some(MMapPageStruct(MMapPage::Cached(elf_page))) =
            exe.inode().as_mappable()?.get_mmap_page(0, true)
        {
            let hdr = unsafe { ElfHeader::load(elf_page.data()) };

            if hdr.is_none() {
                // check for interpreter line and load it
                let data = elf_page.data();

                if !data.starts_with(b"#!") {
                    return None;
                }

                let interp = data
                    .iter()
                    .enumerate()
                    .find(|(_, &e)| e == b'\n')
                    .and_then(|(idx, _)| Some(&data[2..idx]))?;

                let path = core::str::from_utf8(interp)
                    .ok()?
                    .split_whitespace()
                    .collect::<Vec<&str>>();

                if path.is_empty() {
                    return None;
                }
                logln5!("got interp line {}", path[0]);

                let interp = lookup_by_path(&Path::new(path[0]), LookupMode::None).ok()?;

                let (base_addr, entry, elf, tls, _) = self.load_bin(interp.clone())?;

                let exe_args = if path.len() > 1 {
                    let mut ea = ExeArgs::new();
                    for p in &path[1..] {
                        ea.push_back(Box::from(p.as_bytes()))
                    }
                    Some(ea)
                } else {
                    None
                };

                return Some((base_addr, entry, elf, tls, Some((interp, exe_args))));
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
                    if let Ok(interp) = lookup_by_path(&Path::new("/usr/lib/ld.so"), LookupMode::None) {
                        if let Some((_base_addr, entry, _elf, _tls, _)) = self.load_bin(interp) {
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
                        .mmap(
                            Some(virt_begin),
                            len,
                            p.p_flags.into(),
                            MMapFlags::MAP_PRIVATE | MMapFlags::MAP_FIXED,
                            Some(FileHandle::new(0, exe.clone(), OpenFlags::RDONLY, 0)),
                            file_offset as usize,
                        )
                        .expect("Failed to mmap")
                        + len.align_up(PAGE_SIZE);

                    let virt_fend = last_mmap_end;

                    if virt_fend < virt_end {
                        //println!("mmap {} - {} offset {:#x}", virt_fend, virt_end, 0);
                        let len = (virt_end - virt_fend).0;

                        last_mmap_end = self
                            .mmap(
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

            return Some((base_addr, entry_addr, *hdr, tls_vm_info, None));
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
                    f.file.get_fs_dir_item().full_path(),
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
                dbgln!(
                    map,
                    "{} {}: {:?}, {:?} [ {} {:#x} {:#x} ]",
                    e.start,
                    e.end,
                    e.prot,
                    e.flags,
                    f.file.get_fs_dir_item().full_path(),
                    f.starting_offset,
                    f.len,
                );
            } else {
                dbgln!(map, "{} {}: {:?}, {:?}", e.start, e.end, e.prot, e.flags,);
            }
        }
    }

    fn fork(&mut self, vm: &VM) {
        let other = vm.data.lock();

        self.maps = other.maps.clone();
    }

    fn unmap(&mut self, addr: VirtAddr, len: usize) -> bool {
        if !addr.0.is_multiple_of(PAGE_SIZE) {
            return false;
        }
        let start = addr;
        let end = (addr + len).align_up(PAGE_SIZE);

        dbgln!(unmap, "start: {}, end: {}", start, end);

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
                        cursor.insert_merge_after(new_mapping);

                        return true;
                    }
                }
            }
        }

        success
    }

    fn mprotect(&mut self, addr: VirtAddr, len: usize, prot: MMapProt) -> SyscallResult {
        if !addr.0.is_multiple_of(PAGE_SIZE) {
            return Err(SyscallError::EINVAL);
        }

        let start = addr;
        let end = (addr + len).align_up(PAGE_SIZE);

        let mut cursor = self.maps.cursor_front_mut();

        while let Some(c) = cursor.current() {
            if c.end <= start {
                cursor.move_next();
            } else {
                match c.mprotect(start, end, prot)? {
                    MProtectResult::None => {
                        return Ok(0);
                    }
                    MProtectResult::Full => {
                        cursor = cursor.merge_prev_next();
                        cursor.move_next();
                    }
                    MProtectResult::Begin(split) => {
                        cursor.insert_merge_after(split);

                        return Ok(0);
                    }
                    MProtectResult::Mid(split1, split2) => {
                        cursor = cursor.insert_merge_after(split1);
                        cursor.insert_merge_after(split2);

                        return Ok(0);
                    }
                    MProtectResult::End(split) => {
                        cursor = cursor.insert_merge_after(split);
                        cursor.move_next();
                    }
                }
            }
        }

        Ok(0)
    }
}

impl Drop for VMData {
    fn drop(&mut self) {
        dbgln!(map_call, "Drop VMData");
    }
}

pub struct VM {
    data: Mutex<VMData>,
}

impl Drop for VM {
    fn drop(&mut self) {
        dbgln!(map_call, "Drop VM");
    }
}

impl Default for VM {
    fn default() -> VM {
        VM::new()
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
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
        file: Option<Arc<FileHandle>>,
        offset: usize,
    ) -> Option<VirtAddr> {
        let mut data = self.data.lock();

        let res = data.mmap(addr, len, prot, flags, file, offset);

        data.log_vm();

        res
    }

    pub fn munmap(&self, addr: VirtAddr, len: usize) -> bool {
        let mut data = self.data.lock();

        data.unmap(addr, len)
    }

    pub fn mprotect_vm(&self, addr: VirtAddr, len: usize, prot: MMapProt) -> SyscallResult {
        let mut data = self.data.lock();

        data.mprotect(addr, len, prot)
    }

    pub fn handle_pagefault(&self, reason: PageFaultReason, addr: VirtAddr) -> bool {
        if current_task_ref().locks() > 0 {
            logln!("handle_pagefault: locks > 0");
        }
        let mut res = self.data.lock();

        let ret = res.handle_pagefault(reason, addr);

        ret
    }

    pub fn load_bin(
        &self,
        exe: DirEntryItem,
    ) -> Option<(
        VirtAddr,
        VirtAddr,
        ElfHeader,
        Option<TlsVmInfo>,
        Option<(DirEntryItem, Option<ExeArgs>)>,
    )> {
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
