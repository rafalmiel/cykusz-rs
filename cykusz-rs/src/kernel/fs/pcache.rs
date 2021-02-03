use alloc::sync::Arc;
use alloc::sync::Weak;
use core::any::Any;

use spin::Once;

use crate::kernel::fs::cache::{Cache, CacheItem, Cacheable};
use crate::kernel::mm::{allocate_order, deallocate_order, Frame, PhysAddr, PAGE_SIZE};

pub type PageCacheKey = (usize, usize);
type PageCache = Cache<PageCacheKey, PageItemStruct>;

impl Cacheable<PageCacheKey> for PageItemStruct {
    fn cache_key(&self) -> (usize, usize) {
        (self.fs.as_ptr() as *const u8 as usize, self.offset)
    }

    fn deallocate(&self) {
        deallocate_order(&Frame::new(self.page), 0);
    }
}

pub type PageItemInt = CacheItem<PageCacheKey, PageItemStruct>;
pub type PageItem = Arc<PageItemInt>;
pub type PageItemWeak = Weak<PageItemInt>;

#[derive(Clone)]
pub struct PageItemStruct {
    fs: Weak<dyn Any + Send + Sync>,
    offset: usize,
    page: PhysAddr,
}

impl PageItemStruct {
    pub fn make_key(a: &Weak<dyn Any + Send + Sync>, offset: usize) -> PageCacheKey {
        (a.as_ptr() as *const u8 as usize, offset)
    }

    pub fn new(fs: Weak<dyn Any + Send + Sync>, offset: usize) -> PageItemStruct {
        let page = allocate_order(0).unwrap().address();

        PageItemStruct { fs, offset, page }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        unsafe { self.page.to_mapped().as_bytes(PAGE_SIZE) }
    }

    pub fn data_mut(&self) -> &mut [u8] {
        unsafe { self.page.to_mapped().as_bytes_mut(PAGE_SIZE) }
    }
}

static PAGE_CACHE: Once<Arc<PageCache>> = Once::new();

pub fn cache() -> &'static Arc<PageCache> {
    PAGE_CACHE.get().unwrap()
}

pub trait CachedAccess {
    fn read_cached(&self, addr: usize, dest: &mut [u8]) -> Option<usize>;
    fn write_cached(&self, addr: usize, buf: &[u8]) -> Option<usize>;
}

pub trait RawAccess {
    fn read_direct(&self, addr: usize, dest: &mut [u8]) -> Option<usize>;
    fn write_direct(&self, addr: usize, buf: &[u8]) -> Option<usize>;
}

pub fn init() {
    PAGE_CACHE.call_once(|| PageCache::new(256));
}
