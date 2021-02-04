use alloc::sync::Arc;
use alloc::sync::Weak;

use spin::Once;

use crate::kernel::fs::cache::{ArcWrap, Cache, CacheItem, Cacheable, WeakWrap};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, map_flags, map_to_flags, unmap, PhysAddr, PAGE_SIZE};
use crate::kernel::utils::types::Align;

pub type PageCacheKey = (usize, usize);
type PageCache = Cache<PageCacheKey, PageItemStruct>;

impl Cacheable<PageCacheKey> for PageItemStruct {
    fn cache_key(&self) -> (usize, usize) {
        (self.fs.as_ptr() as *const u8 as usize, self.offset)
    }

    fn deallocate(&self) {
        if let Some(cached) = self.fs.upgrade() {
            cached.sync_page(self);
        }
        self.page.to_phys_page().unwrap().unlink_page_cache();
        unmap(self.page.to_virt());
    }
}

pub type PageItemInt = CacheItem<PageCacheKey, PageItemStruct>;
pub type PageItem = ArcWrap<PageItemInt>;
pub type PageItemWeak = WeakWrap<PageItemInt>;

pub struct PageItemStruct {
    fs: Weak<dyn CachedAccess>,
    offset: usize,
    page: PhysAddr,
}

unsafe impl Sync for PageItemStruct {}

impl PageItemStruct {
    pub fn make_key(a: &Weak<dyn CachedAccess>, offset: usize) -> PageCacheKey {
        (a.as_ptr() as *const u8 as usize, offset)
    }

    pub fn new(fs: Weak<dyn CachedAccess>, offset: usize) -> PageItemStruct {
        let page = allocate_order(0).unwrap().address();

        map_to_flags(page.to_virt(), page, PageFlags::WRITABLE);

        PageItemStruct { fs, offset, page }
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

    pub fn notify_dirty(&self, page: &PageItem) {
        map_flags(page.page.to_virt(), PageFlags::WRITABLE);

        if let Some(h) = self.fs.upgrade() {
            h.notify_dirty(page)
        }
    }

    pub fn notify_clean(&self) {
        map_flags(self.page.to_virt(), PageFlags::empty());
    }
}

impl PageItem {
    fn link_with_page(&self) {
        let page = self.page.to_phys_page().unwrap();

        page.link_page_cache(self);

        self.notify_clean();
    }
}

static PAGE_CACHE: Once<Arc<PageCache>> = Once::new();

pub fn cache() -> &'static Arc<PageCache> {
    PAGE_CACHE.get().unwrap()
}

pub trait CachedAccess: RawAccess {
    fn this(&self) -> Weak<dyn CachedAccess>;

    fn notify_dirty(&self, _page: &PageItem);

    fn sync_page(&self, _page: &PageItemStruct);

    fn sync_all(&self);

    fn read_cached(&self, mut sector: usize, dest: &mut [u8]) -> Option<usize> {
        let page_cache = cache();

        let dev = self.this();

        let mut dest_offset = 0;

        while dest_offset < dest.len() {
            let cache_offset = sector / 8;

            if let Some(page) =
                if let Some(page) = page_cache.get(PageItemStruct::make_key(&dev, cache_offset)) {
                    Some(page)
                } else {
                    let new_page = PageItemStruct::new(dev.clone(), cache_offset);

                    self.read_direct(sector.align(8), new_page.data_mut());

                    let page = page_cache.make_item(new_page);

                    page.link_with_page();

                    Some(page)
                }
            {
                use core::cmp::min;

                let page_offset = (sector % 8) * 512;
                let to_copy = min(PAGE_SIZE - page_offset, dest.len() - dest_offset);

                dest[dest_offset..dest_offset + to_copy]
                    .copy_from_slice(&page.data()[page_offset..page_offset + to_copy]);

                dest_offset += to_copy;
                sector = (sector + 8).align(8);
            } else {
                break;
            }
        }

        Some(dest_offset)
    }
    fn write_cached(&self, mut sector: usize, buf: &[u8]) -> Option<usize> {
        let page_cache = cache();

        let dev = self.this();

        let mut copied = 0;

        while copied < buf.len() {
            let cache_offset = sector / 8;

            if let Some(page) =
                if let Some(page) = page_cache.get(PageItemStruct::make_key(&dev, cache_offset)) {
                    Some(page)
                } else {
                    let new_page = PageItemStruct::new(dev.clone(), cache_offset);

                    self.read_direct(sector.align(8), new_page.data_mut());

                    let page = page_cache.make_item(new_page);

                    page.link_with_page();

                    Some(page)
                }
            {
                use core::cmp::min;

                let page_offset = (sector % 8) * 512;
                let to_copy = min(PAGE_SIZE - page_offset, buf.len() - copied);

                page.data_mut()[page_offset..page_offset + to_copy]
                    .copy_from_slice(&buf[copied..copied + to_copy]);

                copied += to_copy;
                sector = (sector + 8).align(8);
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
}

pub fn init() {
    PAGE_CACHE.call_once(|| PageCache::new(256));
}
