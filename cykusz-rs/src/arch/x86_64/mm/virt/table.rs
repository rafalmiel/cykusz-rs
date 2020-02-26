use core::marker::PhantomData;

use crate::arch::mm::virt::entry::Entry;
use crate::arch::x86_64::mm::phys::PhysPage;
use crate::kernel::mm::virt;
use crate::kernel::mm::Frame;
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

    pub fn for_entries(&self, flags: Entry, fun: impl Fn(usize, &Entry)) {
        self.entries
            .iter()
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

    pub fn alloc_next_level(&mut self, idx: usize, user: bool) -> &mut Table<L::NextLevel> {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Table::<L::NextLevel>::new_at_frame_mut(&frame).clear();

            entry.set_frame(&frame);

            entry.inc_entry_count();
        }

        let mut flags = Entry::PRESENT | Entry::WRITABLE;

        if user {
            flags |= Entry::USER;
        }

        entry.set_flags(flags);

        Table::<L::NextLevel>::new_at_frame_mut(&Frame::new(entry.address()))
    }

    pub fn new_mut<'a>(frame: &Frame) -> &'a mut Table<L> {
        Table::<L>::new_at_frame_mut(frame)
    }

    pub fn new<'a>(frame: &Frame) -> &'a Table<L> {
        Table::<L>::new_at_frame(frame)
    }

    pub fn new_mut_at_phys<'a>(addr: PhysAddr) -> &'a mut Table<L> {
        Table::<L>::new_mut(&Frame::new(addr))
    }

    pub fn new_at_phys<'a>(addr: PhysAddr) -> &'a Table<L> {
        Table::<L>::new(&Frame::new(addr))
    }

    pub fn entry_at(&self, idx: usize) -> &Entry {
        return &self.entries[idx];
    }

    pub fn set_entry(&mut self, idx: usize, entry: &Entry) {
        self.entries[idx].set_raw(entry.raw());
    }

    pub fn do_unmap(&mut self, idx: usize) -> bool {
        let entry = &mut self.entries[idx];

        entry.dec_entry_count();

        if entry.get_entry_count() == 0 {
            let frame = Frame::new(entry.address());

            crate::arch::mm::phys::deallocate(&frame);

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
    pub fn set_hugepage(&mut self, idx: usize, frame: &Frame) {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            entry.set_frame_flags(
                &frame,
                Entry::PRESENT | Entry::WRITABLE | Entry::HUGE_PAGE,
            );
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

    pub fn set(&mut self, idx: usize, frame: &Frame) {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            entry.set_frame_flags(&frame, Entry::PRESENT | Entry::WRITABLE);
        }
    }

    pub fn alloc_set_flags(&mut self, idx: usize, flags: Entry) {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Self::new_at_frame_mut(&frame).clear();

            entry.set_frame_flags(&frame, flags | Entry::PRESENT);
        } else {
            let frame = Frame::new(entry.address());
            entry.set_frame_flags(&frame, flags | Entry::PRESENT);
        }
    }

    pub fn set_flags(&mut self, idx: usize, frame: &Frame, flags: Entry) {
        let entry = &mut self.entries[idx];

        if !entry.contains(Entry::PRESENT) {
            entry.set_frame_flags(&frame, Entry::PRESENT | flags);
        }
    }

    pub fn do_unmap(&mut self, idx: usize) -> bool {
        let entry = &mut self.entries[idx];

        if entry.contains(Entry::PRESENT) {
            let frame = Frame::new(entry.address());

            crate::arch::mm::phys::deallocate(&frame);

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

    pub fn map_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(addr);

        let user = page.p4_index() < 256;

        self.alloc_next_level(page.p4_index(), user)
            .alloc_next_level(page.p3_index(), user)
            .alloc_next_level(page.p2_index(), user)
            .alloc_set_flags(page.p1_index(), Entry::from_kernel_flags(flags));
    }

    pub fn map_to_flags(&mut self, virt: VirtAddr, phys: PhysAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        self.alloc_next_level(page.p4_index(), user)
            .alloc_next_level(page.p3_index(), user)
            .alloc_next_level(page.p2_index(), user)
            .set_flags(
                page.p1_index(),
                &Frame::new(phys),
                Entry::from_kernel_flags(flags),
            );
    }

    pub fn map_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        self.alloc_next_level(page.p4_index(), user)
            .alloc_next_level(page.p3_index(), user)
            .alloc_next_level(page.p2_index(), user)
            .set(page.p1_index(), &Frame::new(phys));
    }

    pub fn map_hugepage_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        let user = page.p4_index() < 256;

        self.alloc_next_level(page.p4_index(), user)
            .alloc_next_level(page.p3_index(), user)
            .set_hugepage(page.p2_index(), &Frame::new(phys));
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
        let deallocate_entry = |e: &Entry| {
            let frame = Frame::new(e.address());

            crate::kernel::mm::deallocate(&frame);
        };

        let flags = Entry::PRESENT | Entry::USER;

        self.for_entries(flags, |idx3, e3| {
            let l3 = self.next_level(idx3).unwrap();

            l3.for_entries(flags, |idx2, e2| {
                let l2 = l3.next_level(idx2).unwrap();

                l2.for_entries(flags, |idx1, e1| {
                    let l1 = l2.next_level(idx1).unwrap();

                    l1.for_entries(flags, |_, e| {
                        deallocate_entry(e);
                    });

                    deallocate_entry(e1);
                });

                deallocate_entry(e2);
            });

            deallocate_entry(e3);
        });

        let frame = Frame::new(self.phys_addr());

        crate::kernel::mm::deallocate(&frame);
    }
}
