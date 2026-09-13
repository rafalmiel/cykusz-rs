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
pub use crate::arch::mm::virt::get_flags;
pub use crate::arch::mm::virt::map;
pub use crate::arch::mm::virt::map_flags;
pub use crate::arch::mm::virt::map_to;
pub use crate::arch::mm::virt::map_to_flags;
pub use crate::arch::mm::virt::to_phys;
pub use crate::arch::mm::virt::unmap;
pub use crate::arch::mm::virt::update_flags;
pub use crate::arch::mm::{MappedAddr, PhysAddr, VirtAddr};
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::PerCpu;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Once;
use crate::arch::mm::phys::{DeferredHead, PhysPage};

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
struct DeferredTlbFlush {
    /// (needs flush, frames to dealloc)
    unmaps: PerCpu<(AtomicBool, Spin<DeferredHead>)>,

    in_flush: PerCpu<AtomicBool>,
}

unsafe impl Sync for DeferredTlbFlush {}

struct DeferredTlbFlushGuard<'a> {
    owner: &'a DeferredTlbFlush,
}

impl<'a> Drop for DeferredTlbFlushGuard<'a> {
    fn drop(&mut self) {
        self.owner
            .in_flush
            .this_cpu()
            .store(false, Ordering::Release);
    }
}

impl DeferredTlbFlush {
    fn flush_all(&self, label: &'static str) {
        if self.in_flush.this_cpu().swap(true, Ordering::AcqRel) {
            // Already doing flush, maybe int has fired and we reentered here
            return;
        }
        let _guard = DeferredTlbFlushGuard { owner: self };
        let (flag, unmaps) = self.unmaps.this_cpu();
        let mut frames = {
            if !flag.fetch_and(false, Ordering::AcqRel) {
                return;
            }

            let mut lock = unmaps.lock_irq();

            core::mem::take(&mut *lock)
        };

        dbgln!(ipi_flush, "flush_deferred_frames");

        assert!(crate::kernel::int::is_enabled(), "{}", label);

        // here goes the actual ipi to call flush on other cpus
        crate::kernel::ipi::tlb_flush_all();

        // All cpus have flushed their tlb - safe to deallocate frames now
        frames.drain();
    }

    /// Set flush pending flag and append frames to deallocate
    fn defer_flush_all_frames(&self, dealloc_frames: DeferredHead) {
        let (flag, data) = self.unmaps.this_cpu();
        if dealloc_frames.len() > 0 {
            let mut lock = data.lock_irq();

            lock.push_list(dealloc_frames);
        }

        flag.store(true, Ordering::Release)
    }

    /// Set flush pending flag
    fn defer_flush_all(&self) {
        self.unmaps.this_cpu().0.store(true, Ordering::Release)
    }
}

mod frame;
pub mod heap;
pub mod virt;

static DEFERRED_TLB_FLUSH: Once<DeferredTlbFlush> = Once::new();

pub fn defer_flush_all_frames(mut frames: DeferredHead) {
    if let Some(f) = DEFERRED_TLB_FLUSH.get() {
        f.defer_flush_all_frames(frames);
    } else {
        frames.drain();
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
