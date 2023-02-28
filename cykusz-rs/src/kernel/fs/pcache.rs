use alloc::sync::Arc;
use alloc::sync::Weak;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Once;

use crate::arch::raw::mm::UserAddr;
use crate::kernel::fs::cache::{ArcWrap, Cache, CacheItem, CacheItemAdapter, Cacheable, WeakWrap};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, map_flags, map_to_flags, unmap, PhysAddr, PAGE_SIZE};
use crate::kernel::sched::current_task_ref;
use crate::kernel::sync::Spin;
use crate::kernel::utils::types::Align;

pub type PageCacheKey = (usize, usize);
type PageCache = Cache<PageCacheKey, PageCacheItemStruct>;

impl Cacheable<PageCacheKey> for PageCacheItemStruct {
    fn cache_key(&self) -> (usize, usize) {
        (self.fs.as_ptr() as *const u8 as usize, self.offset)
    }

    fn deallocate(&self, me: &PageCacheItem) {
        if let Some(cached) = self.fs.upgrade() {
            if self.is_dirty() {
                cached.sync_page(me);
            }
        }
        self.page.to_phys_page().unwrap().unlink_page_cache();
        unmap(self.page.to_virt());
    }
}

pub type PageCacheItem = CacheItem<PageCacheKey, PageCacheItemStruct>;
pub type PageCacheItemArc = ArcWrap<PageCacheItem>;
pub type PageCacheItemWeak = WeakWrap<PageCacheItem>;
pub type PageCacheItemAdapter = CacheItemAdapter<PageCacheKey, PageCacheItemStruct>;

pub struct PageDirectItemStruct {
    page: PhysAddr,
    offset: usize,
}

impl PageDirectItemStruct {
    pub fn new(page: PhysAddr, offset: usize) -> PageDirectItemStruct {
        PageDirectItemStruct {
            page, offset
        }
    }

    pub fn page(&self) -> PhysAddr {
        self.page
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

pub struct PageCacheItemStruct {
    fs: Weak<dyn CachedAccess>,
    offset: usize,
    is_dirty: AtomicBool,
    page: PhysAddr,
    user_dirty_mappings: Spin<hashbrown::HashSet<UserAddr>>,
}

unsafe impl Sync for PageCacheItemStruct {}

impl PageCacheItemStruct {
    pub fn make_key(a: &Weak<dyn CachedAccess>, offset: usize) -> PageCacheKey {
        (a.as_ptr() as *const u8 as usize, offset)
    }

    pub fn new(fs: Weak<dyn CachedAccess>, offset: usize) -> PageCacheItemStruct {
        let page = allocate_order(0).unwrap().address();

        map_to_flags(page.to_virt(), page, PageFlags::WRITABLE);

        unsafe {
            page.to_virt().as_bytes_mut(PAGE_SIZE).fill(0);
        }

        PageCacheItemStruct {
            fs,
            offset,
            page,
            is_dirty: AtomicBool::new(false),
            user_dirty_mappings: Spin::new(hashbrown::HashSet::new()),
        }
    }

    pub fn mark_dirty(&self, is: bool) {
        self.is_dirty.store(is, Ordering::SeqCst);
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::SeqCst)
    }

    pub fn sync_to_storage(&self, page: &PageCacheItem) {
        if self.is_dirty() {
            if let Some(cache) = self.fs.upgrade() {
                cache.write_direct(self.offset() * PAGE_SIZE, self.data());
                page.notify_clean(page);
            }
        }
    }

    pub fn flush_to_storage(&self, page: &PageCacheItem) {
        if self.is_dirty() {
            if let Some(cache) = self.fs.upgrade() {
                cache.write_direct_synced(self.offset() * PAGE_SIZE, self.data());
                page.notify_clean(page);
            }
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn page(&self) -> PhysAddr {
        self.page
    }

    pub fn data(&self) -> &[u8] {
        unsafe { self.page.to_virt().as_bytes(PAGE_SIZE) }
    }

    pub fn data_mut(&self) -> &mut [u8] {
        unsafe { self.page.to_virt().as_bytes_mut(PAGE_SIZE) }
    }

    pub fn notify_dirty(&self, page: &PageCacheItemArc, mapping: Option<UserAddr>) {
        logln!("block notify dirty page: {}", self.offset());
        if !page.is_dirty() {
            map_flags(page.page.to_virt(), PageFlags::WRITABLE);

            self.mark_dirty(true);

            if let Some(h) = self.fs.upgrade() {
                h.notify_dirty(page)
            }
        }

        if let Some(mapping) = mapping {
            let mut umaps = self.user_dirty_mappings.lock();

            umaps.insert(mapping);
        }
    }

    pub fn notify_clean(&self, page: &PageCacheItem) {
        logln!("block notify clean page: {}", self.offset());
        self.mark_dirty(false);

        {
            let mut umaps = self.user_dirty_mappings.lock();

            for map in umaps.iter() {
                map.remove_flags(PageFlags::WRITABLE);
            }

            umaps.clear();
        }

        map_flags(self.page.to_virt(), PageFlags::empty());

        if let Some(h) = self.fs.upgrade() {
            h.notify_clean(page)
        }
    }

    pub fn drop_user_addr(&self, mapping: &UserAddr) {
        let mut umaps = self.user_dirty_mappings.lock();

        umaps.remove(mapping);
    }
}

impl PageCacheItemArc {
    fn link_with_page(&self) {
        let page = self.page.to_phys_page().unwrap();

        page.link_page_cache(self);
    }
}

static PAGE_CACHE: Once<Arc<PageCache>> = Once::new();

pub fn cache() -> &'static Arc<PageCache> {
    PAGE_CACHE.get().unwrap()
}

pub trait CachedBlockDev: CachedAccess {
    fn notify_dirty_inode(&self, _page: &PageCacheItemArc);
    fn notify_clean_inode(&self, _page: &PageCacheItem);
    fn sync_all(&self);
}

pub enum MMapPage {
    Cached(PageCacheItemArc),
    Direct(PageDirectItemStruct)
}

pub struct MMapPageStruct(pub MMapPage);

pub trait MappedAccess {
    fn get_mmap_page(&self, offset: usize) -> Option<MMapPageStruct>;
}

impl<T: ?Sized> MappedAccess for T where T: CachedAccess {
    fn get_mmap_page(&self, offset: usize) -> Option<MMapPageStruct> {
        if current_task_ref().locks() > 0 {
            logln!("get_mmap_page: locks > 0");
        }

        let page_cache = cache();

        let dev = self.this();

        let cache_offset = offset / PAGE_SIZE;

        if let Some(page) = page_cache.get(PageCacheItemStruct::make_key(&dev, cache_offset)) {
            Some(MMapPageStruct(MMapPage::Cached(page)))
        } else {
            let new_page = PageCacheItemStruct::new(dev.clone(), cache_offset);

            if let Some(read) = self.read_direct(offset.align(PAGE_SIZE), new_page.data_mut()) {
                if read == 0 {
                    None
                } else {
                    let page = page_cache.make_item(new_page);

                    page.link_with_page();

                    page.notify_clean(&page);

                    Some(MMapPageStruct(MMapPage::Cached(page)))
                }
            } else {
                None
            }
        }
    }

}

pub trait CachedAccess: RawAccess {
    fn this(&self) -> Weak<dyn CachedAccess>;

    fn notify_dirty(&self, _page: &PageCacheItemArc);

    fn notify_clean(&self, _page: &PageCacheItem);

    fn sync_page(&self, page: &PageCacheItem);

    fn try_get_mmap_page(&self, offset: usize) -> Option<PageCacheItemArc> {
        if current_task_ref().locks() > 0 {
            logln!("get_mmap_page: locks > 0");
        }

        let page_cache = cache();

        let dev = self.this();

        let cache_offset = offset / PAGE_SIZE;

        page_cache.get(PageCacheItemStruct::make_key(&dev, cache_offset))
    }

    fn read_cached(&self, mut offset: usize, dest: &mut [u8]) -> Option<usize> {
        let mut dest_offset = 0;

        while dest_offset < dest.len() {
            if current_task_ref().locks() > 0 {
                logln!("read_cached: locks > 0");
            }
            if let Some(MMapPageStruct(MMapPage::Cached(page))) = self.get_mmap_page(offset) {
                use core::cmp::min;

                let page_offset = offset % PAGE_SIZE;
                let to_copy = min(PAGE_SIZE - page_offset, dest.len() - dest_offset);

                dest[dest_offset..dest_offset + to_copy]
                    .copy_from_slice(&page.data()[page_offset..page_offset + to_copy]);

                dest_offset += to_copy;
                offset = (offset + PAGE_SIZE).align(PAGE_SIZE);
            } else {
                break;
            }
        }

        Some(dest_offset)
    }

    fn write_cached(&self, offset: usize, buf: &[u8]) -> Option<usize> {
        self.update_cached(offset, buf)
    }

    fn update_cached(&self, offset: usize, buf: &[u8]) -> Option<usize> {
        self.update_cached_synced(offset, buf, false)
    }

    fn sync_offset(&self, offset: usize) -> bool {
        if let Some(page) = self.try_get_mmap_page(offset) {
            page.flush_to_storage(&page);
            return true;
        }

        false
    }

    fn update_cached_synced(&self, mut offset: usize, buf: &[u8], sync: bool) -> Option<usize> {
        let mut copied = 0;

        while copied < buf.len() {
            if current_task_ref().locks() > 0 {
                logln!("update_cached_synced: locks > 0");
            }
            if let Some(MMapPageStruct(MMapPage::Cached(page))) = self.get_mmap_page(offset) {
                use core::cmp::min;

                let page_offset = offset % PAGE_SIZE;
                let to_copy = min(PAGE_SIZE - page_offset, buf.len() - copied);

                page.data_mut()[page_offset..page_offset + to_copy]
                    .copy_from_slice(&buf[copied..copied + to_copy]);

                copied += to_copy;
                offset = (offset + PAGE_SIZE).align(PAGE_SIZE);

                if sync {
                    page.sync_to_storage(&page);
                }
            } else {
                break;
            }
        }

        Some(copied)
    }
}

pub trait RawAccess: Send + Sync {
    fn read_direct(&self, addr: usize, dest: &mut [u8]) -> Option<usize>;
    fn write_direct(&self, addr: usize, buf: &[u8]) -> Option<usize>;
    fn write_direct_synced(&self, addr: usize, buf: &[u8]) -> Option<usize> {
        self.write_direct(addr, buf)
    }
}

pub fn init() {
    PAGE_CACHE.call_once(|| PageCache::new(256));
}
