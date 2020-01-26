use core::marker::PhantomData;

use crate::arch::mm::virt::entry;
use crate::arch::mm::virt::entry::Entry;
use crate::arch::x86_64::mm::phys::PhysPage;
use crate::kernel::mm::virt;
use crate::kernel::mm::Frame;
use crate::kernel::mm::*;
use crate::kernel::sync::MutexGuard;

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
}

impl<L> Table<L>
where
    L: NotLastLevel,
{
    pub fn next_level_mut(&mut self, idx: usize) -> Option<&mut Table<L::NextLevel>> {
        let entry = &self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            return None;
        }

        Some(Table::<L::NextLevel>::new_at_frame_mut(&Frame::new(
            entry.address(),
        )))
    }

    pub fn alloc_next_level(&mut self, idx: usize) -> &mut Table<L::NextLevel> {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Table::<L::NextLevel>::new_at_frame_mut(&frame).clear();

            entry.set_frame(&frame);
        }

        entry.set_flags(entry::Entry::PRESENT | entry::Entry::WRITABLE | entry::Entry::USER);

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
}

impl<L> Table<L>
where
    L: HugePageLevel,
{
    pub fn set_hugepage(&mut self, idx: usize, frame: &Frame) {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            entry.set_frame_flags(
                &frame,
                entry::Entry::PRESENT | entry::Entry::WRITABLE | entry::Entry::HUGE_PAGE,
            );
        }
    }
}

impl<L> Table<L>
where
    L: LastLevel,
{
    pub fn alloc(&mut self, idx: usize) {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Table::<L>::new_at_frame_mut(&frame).clear();

            entry.set_frame_flags(&frame, entry::Entry::PRESENT | entry::Entry::WRITABLE);
        }
    }

    pub fn set(&mut self, idx: usize, frame: &Frame) {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            entry.set_frame_flags(&frame, entry::Entry::PRESENT | entry::Entry::WRITABLE);
        }
    }

    pub fn alloc_set_flags(&mut self, idx: usize, flags: entry::Entry) {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            let frame = crate::arch::mm::phys::allocate().expect("Out of memory!");

            Table::<L>::new_at_frame_mut(&frame).clear();

            entry.set_frame_flags(&frame, flags | entry::Entry::PRESENT);
        } else {
            let frame = Frame::new(entry.address());
            entry.set_frame_flags(&frame, flags | entry::Entry::PRESENT);
        }
    }

    pub fn set_flags(&mut self, idx: usize, frame: &Frame, flags: entry::Entry) {
        let entry = &mut self.entries[idx];

        if !entry.contains(entry::Entry::PRESENT) {
            entry.set_frame_flags(&frame, entry::Entry::PRESENT | flags);
        }
    }

    pub fn unmap(&mut self, idx: usize) {
        let entry = &mut self.entries[idx];

        if entry.contains(entry::Entry::PRESENT) {
            let frame = Frame::new(entry.address());

            crate::arch::mm::phys::deallocate(&frame);

            entry.clear();
        }
    }
}

impl Table<Level4> {
    fn lock(&self) -> Option<MutexGuard<'static, ()>> {
        if let Some(pp) = self.phys_page() {
            Some(pp.lock_pt())
        } else {
            None
        }
    }

    pub fn map_flags(&mut self, addr: VirtAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(addr);
        self.alloc_next_level(page.p4_index())
            .alloc_next_level(page.p3_index())
            .alloc_next_level(page.p2_index())
            .alloc_set_flags(page.p1_index(), entry::Entry::from_kernel_flags(flags));
    }

    pub fn map_to_flags(&mut self, virt: VirtAddr, phys: PhysAddr, flags: virt::PageFlags) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        self.alloc_next_level(page.p4_index())
            .alloc_next_level(page.p3_index())
            .alloc_next_level(page.p2_index())
            .set_flags(
                page.p1_index(),
                &Frame::new(phys),
                entry::Entry::from_kernel_flags(flags),
            );
    }

    pub fn map_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        self.alloc_next_level(page.p4_index())
            .alloc_next_level(page.p3_index())
            .alloc_next_level(page.p2_index())
            .set(page.p1_index(), &Frame::new(phys));
    }

    pub fn map_hugepage_to(&mut self, virt: VirtAddr, phys: PhysAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        self.alloc_next_level(page.p4_index())
            .alloc_next_level(page.p3_index())
            .set_hugepage(page.p2_index(), &Frame::new(phys));
    }

    pub fn unmap(&mut self, virt: VirtAddr) {
        let _g = self.lock();

        let page = page::Page::new(virt);

        if let Some(p1) = self.next_level_mut(page.p4_index()).and_then(|t| {
            t.next_level_mut(page.p3_index())
                .and_then(|t| t.next_level_mut(page.p2_index()))
        }) {
            p1.unmap(page.p1_index());
        } else {
            println!("ERROR: virt addr {} cannot be unmapped", virt);
        }
    }
}
