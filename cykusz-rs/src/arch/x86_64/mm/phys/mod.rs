use ::alloc::vec::Vec;
use core::sync::atomic::Ordering;
use spin::Once;

use crate::drivers::multiboot2;
use crate::kernel::fs::cache::{ArcWrap, WeakWrap};
use crate::kernel::fs::pcache::{PageCacheItemArc, PageCacheItemWeak};
use crate::kernel::mm::{PAGE_SIZE, PhysAddr};
use crate::kernel::sync::{LockApi, Spin, SpinGuard};

pub use self::alloc::allocate;
pub use self::alloc::allocate_order;
pub use self::alloc::deallocate;
pub use self::alloc::deallocate_order;
pub use self::alloc::free_mem;
pub use self::alloc::order_for_size;
pub use self::alloc::used_mem;

mod alloc;
mod buddy;
mod bump;
mod iter;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct PageKind: u8 {
        const PAGE_CACHE    = 1 << 0;
        const SLAB_META     = 1 << 1;
    }
}

pub struct PhysPageData {
    variant: PhysPageDataVariant,
    flags: PageKind,
}

impl Default for PhysPageData {
    fn default() -> Self {
        PhysPageData {
            variant: PhysPageDataVariant { empty: () },
            flags: PageKind::empty(),
        }
    }
}

impl PhysPageData {
    pub fn as_cache_meta(&mut self) -> &mut PageCacheMeta {
        assert!(self.flags.is_empty() || self.flags.bits() == PageKind::PAGE_CACHE.bits());

        unsafe {
            if self.flags.is_empty() {
                self.variant.cache = core::mem::ManuallyDrop::new(PageCacheMeta {
                    p_cache: PageCacheItemWeak::empty(),
                    vm_use_count: 0,
                });
                self.flags = PageKind::PAGE_CACHE;
            }
            &mut self.variant.cache
        }
    }
}

#[allow(dead_code)]
union PhysPageDataVariant {
    empty: (),
    cache: core::mem::ManuallyDrop<PageCacheMeta>,
    slab: core::mem::ManuallyDrop<SlabMeta>,
}

pub struct PageCacheMeta {
    p_cache: PageCacheItemWeak,
    vm_use_count: u32,
}

impl PageCacheMeta {
    pub fn unlink_page_cache(&mut self) {
        self.p_cache = WeakWrap::empty();
    }

    pub fn link_page_cache(&mut self, page: &PageCacheItemArc) {
        self.p_cache = ArcWrap::downgrade(page);
    }

    pub fn page_item(&self) -> Option<PageCacheItemArc> {
        self.p_cache.upgrade()
    }

    pub fn inc_vm_use_count(&mut self) {
        self.vm_use_count += 1;
    }

    pub fn dec_vm_use_count(&mut self) -> usize {
        if self.vm_use_count > 0 {
            self.vm_use_count -= 1;
        }

        self.vm_use_count as usize
    }

    pub fn vm_use_count(&self) -> usize {
        self.vm_use_count as usize
    }
}

struct SlabMeta {}

#[repr(C)]
pub struct PhysPage {
    pt_lock: Spin<PhysPageData>,
}

unsafe impl Sync for PhysPage {}

impl PhysPage {
    fn base_addr() -> PhysAddr {
        PhysAddr(&pages().unwrap()[0] as *const _ as usize)
    }

    fn this_addr(&self) -> PhysAddr {
        PhysAddr(self as *const _ as usize)
    }

    pub fn to_phys_addr(&self) -> PhysAddr {
        (self.this_addr() - Self::base_addr()) / core::mem::size_of::<Self>() * PAGE_SIZE
    }

    pub fn lock_pt(&self) -> SpinGuard<'_, PhysPageData> {
        self.pt_lock.lock()
    }

    pub fn mark_unused(&self) {
        let mut lock = self.pt_lock.lock();

        lock.variant.empty = ();
        lock.flags = PageKind::empty();
    }
}

impl Default for PhysPage {
    fn default() -> Self {
        PhysPage {
            pt_lock: Spin::new(PhysPageData::default()),
        }
    }
}

pub static PAGES: Once<Vec<PhysPage>> = Once::new();

pub fn pages() -> Option<&'static Vec<PhysPage>> {
    PAGES.get()
}

pub fn init_pages() {
    PAGES.call_once(|| {
        let mut v = Vec::<PhysPage>::new();
        v.resize_with(
            alloc::NUM_PAGES.load(Ordering::SeqCst) as usize,
            Default::default,
        );

        println!(
            "PhysPage size: {} num {}",
            core::mem::size_of::<PhysPage>(),
            v.len()
        );
        v
    });
}

pub fn init(mboot_info: &multiboot2::Info) {
    alloc::init(mboot_info);
}
