pub use self::frame::Frame;
pub use crate::arch::mm::MAX_USER_ADDR;
pub use crate::arch::mm::MMAP_USER_ADDR;
pub use crate::arch::mm::PAGE_SIZE;
pub use crate::arch::mm::phys::allocate;
pub use crate::arch::mm::phys::allocate_order;
pub use crate::arch::mm::phys::allocate_slab;
pub use crate::arch::mm::phys::allocate_slab_zone;
pub use crate::arch::mm::phys::deallocate;
pub use crate::arch::mm::phys::deallocate_order;
pub use crate::arch::mm::phys::free_mem;
pub use crate::arch::mm::phys::free_slab;
pub use crate::arch::mm::phys::used_mem;
use crate::arch::mm::phys::{DeferredHead, PhysPage};
pub use crate::arch::mm::virt::get_flags;
pub use crate::arch::mm::virt::map;
pub use crate::arch::mm::virt::map_flags;
pub use crate::arch::mm::virt::map_to;
pub use crate::arch::mm::virt::map_to_flags;
pub use crate::arch::mm::virt::to_phys;
pub use crate::arch::mm::virt::unmap;
pub use crate::arch::mm::virt::update_flags;
pub use crate::arch::mm::{MappedAddr, PhysAddr, VirtAddr};
use crate::kernel::ipi::IpiTarget;
use crate::kernel::ipi::sync::SyncIpiOp;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::PerCpu;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Once;

#[derive(Copy, Clone)]
pub struct DeferredFrame {
    frame: Frame,
    order: usize,
}

impl DeferredFrame {
    pub fn new(frame: Frame, order: usize) -> Self {
        DeferredFrame { frame, order }
    }

    pub fn deallocate(self) {
        dbgln!(pt, "deferred dealloc {}", self.frame.address());
        deallocate_order(&self.frame, self.order)
    }

    pub fn phys_page(&self) -> &'static PhysPage {
        self.frame.address().to_phys_page().unwrap()
    }

    pub fn order(&self) -> usize {
        self.order
    }
}

#[derive(Default)]
struct DeferredTlbFlushPages {
    pages: [VirtAddr; 32],
    is_flush_all: bool,
    count: usize,
}

impl DeferredTlbFlushPages {
    fn flush_page(&mut self, page: VirtAddr) {
        if self.count >= 32 {
            self.is_flush_all = true;
        } else {
            self.pages[self.count] = page;
            self.count += 1;
        }
    }
}

#[derive(Default)]
struct DeferredTlbFlush {
    /// (needs flush, frames to dealloc)
    unmaps: PerCpu<(AtomicBool, Spin<(DeferredTlbFlushPages, DeferredHead)>)>,

    in_flush: PerCpu<AtomicBool>,
}

unsafe impl Sync for DeferredTlbFlush {}

struct DeferredTlbFlushGuard<'a> {
    owner: &'a DeferredTlbFlush,
}

impl<'a> DeferredTlbFlushGuard<'a> {
    fn try_new(me: &'a DeferredTlbFlush) -> Option<Self> {
        if me.in_flush.this_cpu().swap(true, Ordering::AcqRel) {
            // Already doing flush, maybe int has fired and we reentered here
            return None;
        }
        Some(Self {
            owner: me,
        })
    }
}

impl<'a> Drop for DeferredTlbFlushGuard<'a> {
    fn drop(&mut self) {
        self.owner
            .in_flush
            .this_cpu()
            .store(false, Ordering::Release);
    }
}

fn flush_pages(arg: crate::kernel::ipi::sync::SyncIpiArg) {
    let deferred = unsafe { &*(arg as *const DeferredTlbFlushPages) };

    for i in 0..deferred.count {
        crate::arch::mm::virt::flush(deferred.pages[i]);
    }
}

impl DeferredTlbFlush {
    fn flush_all(&self, label: &'static str) {
        // Clears in_flush flag on drop
        let Some(_guard) = DeferredTlbFlushGuard::try_new(self) else {
            return;
        };

        let (flag, unmaps) = self.unmaps.this_cpu();
        while flag.load(Ordering::Acquire) {
            let (deferred_flush, mut frames) = {
                if !flag.fetch_and(false, Ordering::AcqRel) {
                    return;
                }

                let mut lock = unmaps.lock_irq();

                core::mem::take(&mut *lock)
            };

            dbgln!(ipi_flush, "flush_deferred_frames");

            assert!(crate::kernel::int::is_enabled(), "{}", label);

            // here goes the actual ipi to call flush on other cpus
            if deferred_flush.is_flush_all {
                crate::kernel::ipi::sync::call_sync(IpiTarget::AllButThis, SyncIpiOp::FlushTlbAll);
            } else {
                crate::kernel::ipi::sync::call_sync(
                    IpiTarget::AllButThis,
                    SyncIpiOp::Run(flush_pages, &deferred_flush as *const _ as *mut ()),
                )
            }

            // All cpus have flushed their tlb - safe to deallocate frames now
            frames.drain();
        }
    }

    /// Add page flush addr and append frames to deallocate and set flush pending flag
    fn defer_many_flush_frames(&self, pages: &[VirtAddr], dealloc_frames: DeferredHead) {
        if pages.is_empty() && dealloc_frames.len() == 0 {
            return;
        }
        let (flag, data) = self.unmaps.this_cpu();

        {
            let mut lock = data.lock_irq();
            for p in pages {
                lock.0.flush_page(*p);
            }
            lock.1.push_list(dealloc_frames);
        }

        flag.store(true, Ordering::Release)
    }

    /// Add page flush addr and append frames to deallocate and set flush pending flag
    fn defer_flush_frames(&self, page: VirtAddr, dealloc_frames: DeferredHead) {
        let (flag, data) = self.unmaps.this_cpu();

        {
            let mut lock = data.lock_irq();
            lock.0.flush_page(page);
            lock.1.push_list(dealloc_frames);
        }

        flag.store(true, Ordering::Release)
    }

    /// Set flush pending flag and append frames to deallocate
    fn defer_flush_all_frames(&self, dealloc_frames: DeferredHead) {
        let (flag, data) = self.unmaps.this_cpu();

        let mut lock = data.lock_irq();
        lock.0.is_flush_all = true;
        lock.1.push_list(dealloc_frames);

        flag.store(true, Ordering::Release)
    }

    /// Append frames to deallocate
    fn defer_frames(&self, dealloc_frames: DeferredHead) {
        if dealloc_frames.len() == 0 {
            return;
        }
        let (flag, data) = self.unmaps.this_cpu();

        let mut lock = data.lock_irq();
        lock.1.push_list(dealloc_frames);

        flag.store(true, Ordering::Release)
    }

    /// Add flush page addr and set pending flag
    fn defer_flush(&self, page: VirtAddr) {
        let (flag, data) = self.unmaps.this_cpu();
        {
            let mut lock = data.lock_irq();
            lock.0.flush_page(page);
        }

        flag.store(true, Ordering::Release)
    }

    /// Set flush pending flag
    fn defer_flush_all(&self) {
        let (flag, data) = self.unmaps.this_cpu();
        data.lock_irq().0.is_flush_all = true;
        flag.store(true, Ordering::Release)
    }
}

mod frame;
pub mod heap;
pub mod virt;

static DEFERRED_TLB_FLUSH: Once<DeferredTlbFlush> = Once::new();

pub fn defer_flush_frames(page: VirtAddr, mut frames: DeferredHead) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_flush_frames(page, frames);
    } else {
        frames.drain();
    }
}

pub fn defer_many_flush_frames(page: &[VirtAddr], mut frames: DeferredHead) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_many_flush_frames(page, frames);
    } else {
        frames.drain();
    }
}

pub fn defer_flush_all_frames(mut frames: DeferredHead) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_flush_all_frames(frames);
    } else {
        frames.drain();
    }
}

pub fn defer_frames(mut frames: DeferredHead) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_frames(frames);
    } else {
        frames.drain();
    }
}

pub fn defer_flush(page: VirtAddr) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_flush(page);
    }
}

pub fn defer_flush_all() {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_flush_all();
    }
}

pub fn flush_deferred_frames(label: &'static str) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.flush_all(label);
    }
}

pub fn init() {
    heap::init();
}

pub fn smp_init_deferred() {
    DEFERRED_TLB_FLUSH.call_once(|| DeferredTlbFlush::default());
}
