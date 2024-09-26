use core::marker::PhantomData;

use crate::arch::mm::virt::entry::Entry;
use crate::arch::x86_64::mm::phys::PhysPage;
use crate::kernel::mm::*;
use crate::kernel::sync::SpinGuard;

use super::page;

const ENTRIES_COUNT: usize = 512;

pub enum Level4 {}

#[allow(dead_code)]
pub enum Level3 {}

#[allow(dead_code)]
pub enum Level2 {}

pub enum Level1 {}

pub trait NotLastLevel: TableLevel {
    type NextLevel: TableLevel;
}

pub trait TableLevel {}

pub trait LastLevel: TableLevel {}

pub trait HugePageLevel: TableLevel {}

pub trait TopLevel: TableLevel {}

impl TableLevel for Level4 {}

impl TableLevel for Level3 {}

impl TableLevel for Level2 {}

impl TableLevel for Level1 {}

impl LastLevel for Level1 {}

impl TopLevel for Level4 {}

impl HugePageLevel for Level2 {}

impl NotLastLevel for Level4 {
    type NextLevel = Level3;
}

impl NotLastLevel for Level3 {
    type NextLevel = Level2;
}

impl NotLastLevel for Level2 {
    type NextLevel = Level1;
}

pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRIES_COUNT],
    level: PhantomData<L>,
}

pub type P4Table = Table<Level4>;

impl<L> Table<L>
where
    L: TableLevel,
{
    fn new_at_frame_mut<'a>(frame: &Frame) -> &'a mut Table<L> {
        unsafe { frame.address_mapped().read_mut::<Table<L>>() }
    }

    fn new_at_frame<'a>(frame: &Frame) -> &'a Table<L> {
        unsafe { frame.address_mapped().read_ref::<Table<L>>() }
    }

    pub fn new_alloc<'a>() -> &'a mut Table<L> {
        let mut frame = crate::kernel::mm::allocate().unwrap();
        frame.clear();

        Self::new_at_frame_mut(&frame)
    }

    pub fn clear(&mut self) {
        for i in 0..ENTRIES_COUNT {
            self.entries[i].clear();
        }
    }

    pub fn phys_addr(&self) -> PhysAddr {
        MappedAddr(self as *const _ as usize).to_phys()
    }

    pub fn phys_page(&self) -> Option<&'static PhysPage> {
        self.phys_addr().to_phys_page()
    }

    pub fn for_entries(&self, flags: Entry, mut fun: impl FnMut(usize, &Entry)) {
        self.entries
            .iter()
            .enumerate()
            .filter(|e| e.1.contains(flags))
            .for_each(|(idx, e)| {
                fun(idx, e);
            })
    }

    pub fn entry_at(&self, idx: usize) -> &Entry {
        return &self.entries[idx];
    }

    pub fn entry_at_mut(&mut self, idx: usize) -> &mut Entry {
        return &mut self.entries[idx];
    }

    pub fn set_entry(&mut self, idx: usize, entry: &Entry) {
        self.entries[idx].set_raw(entry.raw());
    }
}

impl Table<Level1> {
    pub fn for_entries_mut(&mut self, flags: Entry, mut fun: impl FnMut(usize, &mut Entry)) {
        self.entries
            .iter_mut()
            .enumerate()
            .filter(|e| e.1.contains(flags))
            .for_each(|(idx, e)| {
                fun(idx, e);
            })
    }
}

impl<L> Table<L>
where
    L: NotLastLevel,
{
    pub fn for_entries_mut(
        &mut self,
        flags: Entry,
        mut fun: impl FnMut(usize, &mut Entry, &mut Table<L::NextLevel>),
    ) {
        self.entries
            .iter_mut()
            .enumerate()
            .filter(|e| e.1.contains(flags))
            .for_each(|(idx, e)| {
                let lvl = Table::<L::NextLevel>::new_at_frame_mut(&Frame::new(e.address()));
                fun(idx, e, lvl);
            })
    }

    pub fn next_level_mut(&mut self, idx: usize) -> Option<&mut Table<L::NextLevel>> {
        let entry = &self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            return None;
        }

        Some(Table::<L::NextLevel>::new_at_frame_mut(&Frame::new(
            entry.address(),
        )))
    }

    pub fn next_level(&self, idx: usize) -> Option<&Table<L::NextLevel>> {
        let entry = &self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            return None;
        }

        Some(Table::<L::NextLevel>::new_at_frame(&Frame::new(
            entry.address(),
        )))
    }

    pub fn alloc_next_level(&mut self, idx: usize, user: bool) -> (bool, &mut Table<L::NextLevel>) {
        let entry = &mut self.entries[idx];

        let was_alloc = if !entry.contains(Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Table::<L::NextLevel>::new_at_frame_mut(&frame).clear();

            entry.set_frame(&frame);

            true
        } else {
            false
        };

        let mut flags = Entry::PRESENT | Entry::WRITABLE;

        if user {
            flags |= Entry::USER;
        }

        entry.set_flags(flags);

        (
            was_alloc,
            Table::<L::NextLevel>::new_at_frame_mut(&Frame::new(entry.address())),
        )
    }

    pub fn new_from_frame_mut<'a>(frame: &Frame) -> &'a mut Table<L> {
        Table::<L>::new_at_frame_mut(frame)
    }

    pub fn new_from_frame<'a>(frame: &Frame) -> &'a Table<L> {
        Table::<L>::new_at_frame(frame)
    }

    pub fn new_mut_at_phys<'a>(addr: PhysAddr) -> &'a mut Table<L> {
        Table::<L>::new_from_frame_mut(&Frame::new(addr))
    }

    pub fn new_at_phys<'a>(addr: PhysAddr) -> &'a Table<L> {
        Table::<L>::new_from_frame(&Frame::new(addr))
    }

    pub fn do_unmap(&mut self, idx: usize) -> bool {
        let entry = &mut self.entries[idx];

        entry.dec_entry_count();

        if entry.get_entry_count() == 0 {
            entry.unref_phys_page();

            entry.clear();

            return true;
        }

        false
    }
}

impl<L> Table<L>
where
    L: HugePageLevel,
{
    pub fn set_hugepage(&mut self, idx: usize, frame: &Frame) -> bool {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            entry.set_frame_flags(&frame, Entry::PRESENT | Entry::WRITABLE | Entry::HUGE_PAGE);

            true
        } else {
            false
        }
    }
}

impl Table<Level1> {
    pub fn alloc(&mut self, idx: usize) {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Self::new_at_frame_mut(&frame).clear();

            entry.set_frame_flags(&frame, Entry::PRESENT | Entry::WRITABLE);
        }
    }

    pub fn set(&mut self, idx: usize, frame: &Frame) -> bool {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            entry.set_frame_flags(&frame, Entry::PRESENT | Entry::WRITABLE);

            true
        } else {
            false
        }
    }

    pub fn alloc_set_flags(&mut self, idx: usize, flags: Entry) -> bool {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            let mut frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            frame.clear();

            entry.set_frame_flags(&frame, flags | Entry::PRESENT);

            true
        } else {
            let frame = Frame::new(entry.address());
            entry.set_frame_flags(&frame, flags | Entry::PRESENT);

            false
        }
    }

    pub fn set_flags(&mut self, idx: usize, frame: &Frame, flags: Entry) -> bool {
        let entry = &mut self.entries[idx];

        let inc = !entry.contains(Entry::PRESENT);

        entry.set_frame_flags(frame, Entry::PRESENT | flags);

        inc
    }

    pub fn do_unmap(&mut self, idx: usize) -> bool {
        let entry = &mut self.entries[idx];

        if entry.contains(Entry::PRESENT) {
            entry.unref_phys_page();

            entry.clear();

            return true;
        }

        false
    }
}

impl Table<Level4> {
    fn lock(&self) -> Option<SpinGuard<'static, ()>> {
        if let Some(pp) = self.phys_page() {
            Some(pp.lock_pt())
        } else {
            None
        }
    }

    pub fn new<'a>() -> &'a mut Table<Level4> {
        let table = Self::new_alloc();

        table.clear();

        table.ref_table();

        table
    }

    pub fn ref_table(&self) {
        if let Some(page) = self.phys_page() {
            page.inc_vm_use_count();
        }
    }

    pub fn unref_table(&self) {
        if let Some(page) = self.phys_page() {
            if page.dec_vm_use_count() == 0 {
                let frame = Frame::new(self.phys_addr());

                crate::kernel::mm::deallocate(&frame);
            }
        }
    }

    pub fn unref_table_with(&mut self, f: impl Fn(&mut P4Table)) {
        if let Some(page) = self.phys_page() {
            if page.dec_vm_use_count() == 0 {
                f(self);

                let frame = Frame::new(self.phys_addr());

                crate::kernel::mm::deallocate(&frame);
            }
        }
    }

    pub fn to_phys(&self, addr: VirtAddr) -> Option<PhysAddr> {
        let _g = self.lock();

        let page = page::Page::new(addr);

        let l3 = self.next_level(page.p4_index())?;
        let entry3 = l3.entry_at(page.p3_index());

        let l2 = if entry3.contains(Entry::HUGE_PAGE | Entry::PRESENT) {
            return Some(entry3.address() + (addr.0 & 0x3FFFFFFF));
        } else {
            l3.next_level(page.p3_index())?
        };

        let entry2 = l2.entry_at(page.p2_index());

        let l1 = if entry2.contains(Entry::HUGE_PAGE | Entry::PRESENT) {
            return Some(entry2.address() + (addr.0 & 0x1FFFFF));
        } else {
            l2.next_level(page.p2_index())?
        };

        let entry = l1.entry_at(page.p1_index());

        if entry.contains(Entry::PRESENT) {
            Some(entry.address() + (addr.0 & 0xFFF))
        } else {
            None
        }
    }

    pub fn is_mapped(&self, addr: VirtAddr) -> bool {
        self.to_phys(addr).is_some()
    }

    fn change_entry(
        &mut self,
        addr: VirtAddr,
        mut fun: impl FnMut(&mut Entry),
    ) -> Option<PhysAddr> {
        let _g = self.lock();

        let page = page::Page::new(addr);

        let l3 = self.next_level_mut(page.p4_index())?;
        let entry3 = l3.entry_at_mut(page.p3_index());

        let l2 = if entry3.contains(Entry::HUGE_PAGE | Entry::PRESENT) {
            fun(entry3);
            return Some(entry3.address() + (addr.0 & 0x3FFFFFFF));
        } else {
            l3.next_level_mut(page.p3_index())?
        };

        let entry2 = l2.entry_at_mut(page.p2_index());

        let l1 = if entry2.contains(Entry::HUGE_PAGE | Entry::PRESENT) {
            fun(entry2);

            return Some(entry2.address() + (addr.0 & 0x1FFFFF));
        } else {
            l2.next_level_mut(page.p2_index())?
        };

        let entry = l1.entry_at_mut(page.p1_index());

        return if entry.contains(Entry::PRESENT) {
            fun(entry);
            Some(entry.address() + (addr.0 & 0xFFF))
        } else {
            None
        };
    }

    pub fn update_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) -> Option<PhysAddr> {
        self.change_entry(addr, |e| {
            e.set_flags(Entry::PRESENT | Entry::from_kernel_flags(flags));
        })
    }

    pub fn remove_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) -> Option<PhysAddr> {
        self.change_entry(addr, |e| {
            e.remove(Entry::from_kernel_flags(flags));
        })
    }

    pub fn insert_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) -> Option<PhysAddr> {
        self.change_entry(addr, |e| {
            e.insert(Entry::from_kernel_flags(flags));
        })
    }

    pub fn get_flags(&self, addr: VirtAddr) -> Option<Entry> {
        let _g = self.lock();

        let page = page::Page::new(addr);

        if let Some(l3) = self.next_level(page.p4_index()) {
            if let Some(l2) = l3.next_level(page.p3_index()) {
                if let Some(l1) = l2.next_level(page.p2_index()) {
                    return Some(*l1.entry_at(page.p1_index()));
                }
            }
        }

        None
    }

    pub fn map_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(addr);

        let user = page.p4_index() < 256;

        let (_, l3) = self.alloc_next_level(page.p4_index(), user);

        let (was_alloc_3, l2) = l3.alloc_next_level(page.p3_index(), user);

        let (was_alloc_2, l1) = l2.alloc_next_level(page.p2_index(), user);

        dbgln!(virt, "map_flags {} {:?}", addr, flags);
        if l1.alloc_set_flags(page.p1_index(), Entry::from_kernel_flags(flags)) {
            l2.entries[page.p2_index()].inc_entry_count();
        }

        if was_alloc_2 {
            l3.entries[page.p3_index()].inc_entry_count();
        }

        if was_alloc_3 {
            self.entries[page.p4_index()].inc_entry_count();
        }
    }

    pub fn map_to_flags(&mut self, virt: VirtAddr, phys: PhysAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        let (_, l3) = self.alloc_next_level(page.p4_index(), user);

        let (was_alloc_3, l2) = l3.alloc_next_level(page.p3_index(), user);

        let (was_alloc_2, l1) = l2.alloc_next_level(page.p2_index(), user);

        if l1.set_flags(
            page.p1_index(),
            &Frame::new(phys),
            Entry::from_kernel_flags(flags),
        ) {
            l2.entries[page.p2_index()].inc_entry_count();
        }

        if was_alloc_2 {
            l3.entries[page.p3_index()].inc_entry_count();
        }

        if was_alloc_3 {
            self.entries[page.p4_index()].inc_entry_count();
        }
    }

    pub fn map_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        let (_, l3) = self.alloc_next_level(page.p4_index(), user);

        let (was_alloc_3, l2) = l3.alloc_next_level(page.p3_index(), user);

        let (was_alloc_2, l1) = l2.alloc_next_level(page.p2_index(), user);

        if l1.set(page.p1_index(), &Frame::new(phys)) {
            l2.entries[page.p2_index()].inc_entry_count();
        }

        if was_alloc_2 {
            l3.entries[page.p3_index()].inc_entry_count();
        }

        if was_alloc_3 {
            self.entries[page.p4_index()].inc_entry_count();
        }
    }

    pub fn map_hugepage_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        let (_, l3) = self.alloc_next_level(page.p4_index(), user);

        let (was_alloc_3, l2) = l3.alloc_next_level(page.p3_index(), user);

        if l2.set_hugepage(page.p2_index(), &Frame::new(phys)) {
            l3.entries[page.p3_index()].inc_entry_count();
        }

        if was_alloc_3 {
            self.entries[page.p4_index()].inc_entry_count();
        }
    }

    pub fn unmap(&mut self, virt: VirtAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        if let Some(l3) = self.next_level_mut(page.p4_index()) {
            if let Some(l2) = l3.next_level_mut(page.p3_index()) {
                if let Some(l1) = l2.next_level_mut(page.p2_index()) {
                    if l1.do_unmap(page.p1_index()) {
                        if l2.do_unmap(page.p2_index()) {
                            l3.do_unmap(page.p3_index());
                        }
                    }
                }
            }
        }
    }

    pub fn deallocate_user(&mut self) {
        let _g = self.lock();

        let flags = Entry::PRESENT | Entry::USER;

        self.for_entries_mut(flags, |_idx3, e3, l3| {
            l3.for_entries_mut(flags, |_idx2, e2, l2| {
                l2.for_entries_mut(flags, |_idx1, e1, l1| {
                    l1.for_entries_mut(flags, |_idx, e| {
                        e.unref_phys_page();
                        e.clear();
                    });

                    e1.unref_phys_page();
                    e1.clear();
                });

                e2.unref_phys_page();
                e2.clear();
            });

            e3.unref_phys_page();
            e3.clear();
        });
    }

    pub fn duplicate(&mut self) -> &P4Table {
        let _g = self.lock();

        let new = P4Table::new();

        for e in 256..512 {
            new.set_entry(e, self.entry_at(e));
        }

        let flags = Entry::PRESENT | Entry::USER;

        self.for_entries_mut(flags, |idx4, _e4, l3| {
            let n3 = new.alloc_next_level(idx4, true).1;

            let mut count_3 = 0;

            l3.for_entries_mut(flags, |idx3, _e3, l2| {
                let (w2, n2) = n3.alloc_next_level(idx3, true);

                if w2 {
                    count_3 += 1;
                }

                let mut count_2 = 0;

                l2.for_entries_mut(flags, |idx2, _e2, l1| {
                    let (w1, n1) = n2.alloc_next_level(idx2, true);

                    if w1 {
                        count_2 += 1;
                    }

                    let mut count_1 = 0;

                    l1.for_entries_mut(flags, |idx1, e1| {
                        // Setup copy on write page
                        e1.remove(Entry::WRITABLE);
                        n1.set_flags(idx1, &Frame::new(e1.address()), *e1);

                        count_1 += 1;
                    });

                    n2.entries[idx2].set_entry_count(count_1);
                });

                n3.entries[idx3].set_entry_count(count_2);
            });

            new.entries[idx4].set_entry_count(count_3);
        });

        new
    }
}
