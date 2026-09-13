use crate::drivers::multiboot2;
use crate::kernel::fs::cache::{ArcWrap, WeakWrap};
use crate::kernel::fs::pcache::{PageCacheItemArc, PageCacheItemWeak};
use crate::kernel::mm::{Frame, PAGE_SIZE, PhysAddr};
use crate::kernel::sync::{LockApi, Spin, SpinGuard};
use ::alloc::vec::Vec;
use core::fmt::Formatter;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Once;

pub use self::alloc::allocate;
pub use self::alloc::allocate_order;
pub use self::alloc::allocate_order_zone;
pub use self::alloc::allocate_slab;
pub use self::alloc::allocate_slab_zone;
pub use self::alloc::allocate_zone;
pub use self::alloc::deallocate;
pub use self::alloc::deallocate_order;
pub use self::alloc::free_mem;
pub use self::alloc::free_slab;
pub use self::alloc::order_for_size;
pub use self::alloc::used_mem;

mod alloc;
mod buddy;
mod bump;
mod iter;
mod slab;

#[allow(dead_code)]
#[derive(Debug, Default, Copy, Clone)]
#[repr(u8)]
pub enum MemZone {
    ZoneDma = 0,   // Below 16MB
    ZoneDma32 = 1, // Below 4GB
    #[default]
    ZoneNormal = 2, // All the rest
}

impl MemZone {
    fn in_zone(&self, addr: PhysAddr) -> bool {
        match self {
            MemZone::ZoneDma => addr.0 <= 16 * 1024 * 1024,
            MemZone::ZoneDma32 => addr.0 <= 4 * 1024 * 1024 * 1024,
            MemZone::ZoneNormal => true,
        }
    }
}

pub struct PhysPageData {
    variant: PhysPageDataVariant,
}

/// Enum to save space as we aim for 32 bytes PhysPage size limit
/// PhysPage is used either for PageCache or Slab allocator but never both
enum PhysPageDataVariant {
    Empty,
    Cache(PageCacheMeta),
    Slab(SlabMeta),
    Deferred(DeferredMeta),
}

impl core::fmt::Display for PhysPageDataVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            PhysPageDataVariant::Empty => f.write_str("Empty"),
            PhysPageDataVariant::Cache(_) => f.write_str("Cache"),
            PhysPageDataVariant::Slab(_) => f.write_str("Slab"),
            PhysPageDataVariant::Deferred(_) => f.write_str("Deferred"),
        }
    }
}

impl Default for PhysPageData {
    fn default() -> Self {
        PhysPageData {
            variant: PhysPageDataVariant::Empty,
        }
    }
}

impl PhysPageData {
    pub fn try_as_cache_meta(&mut self) -> Option<&mut PageCacheMeta> {
        if let PhysPageDataVariant::Cache(cache) = &mut self.variant {
            Some(cache)
        } else {
            None
        }
    }

    pub fn as_cache_meta(&mut self) -> &mut PageCacheMeta {
        //dbgln!(phys_page, "{:#p} -> Cache, {}", core::ptr::addr_of!(self), self.variant);
        assert!(matches!(
            self.variant,
            PhysPageDataVariant::Empty | PhysPageDataVariant::Cache(_)
        ));

        if let PhysPageDataVariant::Empty = self.variant {
            self.variant = PhysPageDataVariant::Cache(PageCacheMeta {
                p_cache: PageCacheItemWeak::empty(),
            });
        }
        let PhysPageDataVariant::Cache(cache) = &mut self.variant else {
            panic!("as_cache_meta: invalid PhysPageData variant");
        };

        cache
    }

    pub fn as_slab_meta(&mut self) -> &mut SlabMeta {
        //dbgln!(phys_page, "{:#p} -> Slab, {}", core::ptr::addr_of!(self), self.variant);
        assert!(matches!(
            self.variant,
            PhysPageDataVariant::Empty | PhysPageDataVariant::Slab(_)
        ));

        if let PhysPageDataVariant::Empty = self.variant {
            self.variant = PhysPageDataVariant::Slab(SlabMeta::default());
        }
        let PhysPageDataVariant::Slab(slab) = &mut self.variant else {
            panic!("as_slab_meta: invalid PhysPageData variant");
        };

        slab
    }

    pub fn as_deferred_meta(&mut self) -> &mut DeferredMeta {
        //dbgln!(phys_page, "{:#p} -> Slab, {}", core::ptr::addr_of!(self), self.variant);
        assert!(matches!(
            self.variant,
            PhysPageDataVariant::Empty | PhysPageDataVariant::Deferred(_)
        ));

        if let PhysPageDataVariant::Empty = self.variant {
            self.variant = PhysPageDataVariant::Deferred(DeferredMeta::default());
        }
        let PhysPageDataVariant::Deferred(deferred) = &mut self.variant else {
            panic!("as_deferred_meta: invalid PhysPageData variant");
        };

        deferred
    }
}

pub struct PageCacheMeta {
    p_cache: PageCacheItemWeak,
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
}

#[repr(C)]
#[derive(Default)]
/// Slab allocator metadata
pub struct SlabMeta {
    /// Pointer to the first free chunk
    free_head: crate::kernel::mm::VirtAddr,
    /// Chunk size of this slab
    chunk_size: u16,
    /// Is it dma or normal memory zone? dma slabs are stored separately
    /// as they have different page table mappings
    mem_zone: MemZone,
    /// Number of free chunks in this slab
    free_count: u16,
}

/// Head of a singly linked list of pages to deallocate
#[derive(Default)]
pub struct DeferredHead {
    next: u32,
    last: u32,
    len: usize,
}

impl DeferredHead {
    pub fn push(&mut self, page: &'static PhysPage, order: u8) {
        {
            let mut lock = page.lock_pt();
            let deferred = lock.as_deferred_meta();

            deferred.next = self.next;
            deferred.order = order;
        }
        self.next = page.index() as u32;
        if self.last == 0 {
            self.last = self.next;
        }
        self.len += 1;
    }

    pub fn push_list(&mut self, other: DeferredHead) {
        if other.next == 0 {
            // The other is empty, do nothing
            return;
        }

        if self.next == 0 {
            // Our list is empty, just take the other
            *self = other;
            return;
        }

        // Both lists are not empty - point our tail to the other list head
        let tail_page = PhysPage::from_index(self.last as usize);
        {
            let mut lock = tail_page.lock_pt();
            lock.as_deferred_meta().next = other.next;
        }
        self.last = other.last;
        self.len += other.len;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Deallocate all frames in this list
    pub fn drain(&mut self) {
        let mut current = self.next;
        self.next = 0;
        self.last = 0;
        self.len = 0;

        while current != 0 {
            let page = PhysPage::from_index(current as usize);

            let (next, order) = {
                let mut lock = page.lock_pt();

                let (next, order) = {
                    let deferred = lock.as_deferred_meta();

                    (deferred.next, deferred.order)
                };

                lock.variant = PhysPageDataVariant::Empty;

                (next, order)
            };
            deallocate_order(&Frame::new(page.to_phys_addr()), order as usize);
            current = next;
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct DeferredMeta {
    next: u32,
    order: u8,
}

#[repr(C)]
pub struct PhysPage {
    pt_lock: Spin<PhysPageData>,
    // vm_use_count is 8 bytes, but we are still within 32bytes limit size of PhysPage size
    // As 8 bytes is probably excessive, it allows us to add more flags there in the future
    vm_use_count: AtomicU64,
}

const _: () = assert!(32 == core::mem::size_of::<crate::arch::mm::phys::PhysPage>());

unsafe impl Sync for PhysPage {}

impl PhysPage {
    fn from_index(index: usize) -> &'static PhysPage {
        assert_ne!(index, 0, "from index sentiel value");
        &pages().unwrap()[index]
    }

    fn base_addr() -> PhysAddr {
        PhysAddr(&pages().unwrap()[0] as *const _ as usize)
    }

    fn this_addr(&self) -> PhysAddr {
        PhysAddr(self as *const _ as usize)
    }

    pub fn to_phys_addr(&self) -> PhysAddr {
        (self.this_addr() - Self::base_addr()) / core::mem::size_of::<Self>() * PAGE_SIZE
    }

    pub fn index(&self) -> usize {
        (self.this_addr().0 - Self::base_addr().0) / size_of::<Self>()
    }

    pub fn lock_pt(&self) -> SpinGuard<'_, PhysPageData> {
        self.pt_lock.lock()
    }

    pub fn mark_unused(&self) {
        let mut lock = self.lock_pt();

        lock.variant = PhysPageDataVariant::Empty
    }

    pub fn inc_vm_use_count(&self) {
        self.vm_use_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_vm_use_count(&self) -> usize {
        self.vm_use_count.fetch_sub(1, Ordering::Relaxed) as usize - 1
    }

    pub fn vm_use_count(&self) -> usize {
        self.vm_use_count.load(Ordering::Relaxed) as usize
    }
}

impl Default for PhysPage {
    fn default() -> Self {
        PhysPage {
            pt_lock: Spin::new(PhysPageData::default()),
            vm_use_count: AtomicU64::default(),
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

        v
    });
}

pub fn init(mboot_info: &multiboot2::Info) {
    alloc::init(mboot_info);
}
