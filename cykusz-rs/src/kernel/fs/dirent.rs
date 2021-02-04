use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use spin::Once;

use crate::kernel::fs::cache::{ArcWrap, Cache, CacheItem, Cacheable};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::INodeItem;
use crate::kernel::sync::{RwSpin, RwSpinReadGuard, RwSpinWriteGuard};

type CacheKey = (usize, String);

impl Cacheable<CacheKey> for DirEntry {
    fn cache_key(&self) -> CacheKey {
        let data = self.data.read();
        Self::make_key(data.parent.as_ref(), &data.name)
    }

    fn make_unused(&self, _new_ref: &Weak<CacheItem<CacheKey, DirEntry>>) {
        self.data.write().parent = None;
    }

    fn deallocate(&self) {}
}

pub type DirEntryItem = ArcWrap<CacheItem<CacheKey, DirEntry>>;

static CACHE_MARKER_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn new_cache_marker() -> usize {
    CACHE_MARKER_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub struct DirEntryData {
    pub parent: Option<DirEntryItem>,
    pub name: String,
    pub inode: INodeItem,
}

impl Drop for DirEntryData {
    fn drop(&mut self) {
        self.name.clear();
        self.name.shrink_to_fit();
    }
}

pub struct DirEntry {
    data: RwSpin<DirEntryData>,
    mountpoint: AtomicBool,
    fs: Once<Weak<dyn Filesystem>>,
    cache_marker: usize,
}

impl DirEntry {
    pub fn new_root(inode: INodeItem, name: String) -> DirEntryItem {
        cache().make_item_no_cache(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: None,
                name,
                inode: inode.clone(),
            }),
            mountpoint: AtomicBool::new(false),
            fs: Once::new(),
            cache_marker: new_cache_marker(),
        })
    }

    pub fn new(parent: DirEntryItem, inode: INodeItem, name: String) -> DirEntryItem {
        if let Some(e) = crate::kernel::fs::dirent::get(parent.clone(), &name) {
            return e;
        } else {
            let do_cache = ![".", ".."].contains(&name.as_str());

            let e = DirEntry {
                data: RwSpin::new(DirEntryData {
                    parent: Some(parent.clone()),
                    name,
                    inode: inode.clone(),
                }),
                mountpoint: AtomicBool::new(false),
                fs: Once::initialized(inode.fs()),
                cache_marker: if do_cache { new_cache_marker() } else { 0 },
            };

            let res = if do_cache {
                cache().make_item(e)
            } else {
                cache().make_item_no_cache(e)
            };

            res
        }
    }

    pub fn new_no_cache(parent: DirEntryItem, inode: INodeItem, name: String) -> DirEntryItem {
        cache().make_item_no_cache(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: Some(parent),
                name,
                inode: inode.clone(),
            }),
            mountpoint: AtomicBool::new(false),
            fs: Once::initialized(inode.fs()),
            cache_marker: 0,
        })
    }

    pub fn inode_wrap(inode: INodeItem) -> DirEntryItem {
        cache().make_item_no_cache(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: None,
                name: String::new(),
                inode: inode.clone(),
            }),
            mountpoint: AtomicBool::new(false),
            fs: Once::initialized(inode.fs()),
            cache_marker: 0,
        })
    }

    pub fn read(&self) -> RwSpinReadGuard<DirEntryData> {
        self.data.read()
    }

    pub fn write(&self) -> RwSpinWriteGuard<DirEntryData> {
        self.data.write()
    }

    pub fn name(&self) -> String {
        self.data.read().name.clone()
    }

    pub fn inode_id(&self) -> usize {
        self.data.read().inode.id().unwrap()
    }

    pub fn inode(&self) -> INodeItem {
        self.data.read().inode.clone()
    }

    pub fn make_key(parent: Option<&DirEntryItem>, name: &String) -> CacheKey {
        if let Some(p) = parent {
            (p.cache_marker, name.clone())
        } else {
            (0, name.clone())
        }
    }

    pub fn parent(&self) -> Option<DirEntryItem> {
        self.data.read().parent.clone()
    }

    pub fn update_inode(&self, inode: INodeItem) {
        self.data.write().inode = inode;
    }

    pub fn update_parent(&self, parent: Option<DirEntryItem>) {
        self.data.write().parent = parent;
    }

    pub fn update_name(&self, name: String) {
        self.data.write().name = name;
    }

    pub fn is_mountpoint(&self) -> bool {
        self.mountpoint.load(Ordering::SeqCst)
    }

    pub fn set_is_mountpont(&self, is: bool) {
        self.mountpoint.store(is, Ordering::SeqCst);
    }

    pub fn is_valid(&self) -> bool {
        Weak::strong_count(&self.fs.get().unwrap()) > 0
    }

    pub fn init_fs(&self, fs: Weak<dyn Filesystem>) {
        self.fs.call_once(|| fs);
    }

    pub fn drop_from_cache(&self) {
        cache().remove(&self.cache_key());
        crate::kernel::fs::icache::cache().remove(&self.inode().cache_key());
    }
}

static CACHE: Once<Arc<Cache<CacheKey, DirEntry>>> = Once::new();

pub fn cache() -> &'static Arc<Cache<CacheKey, DirEntry>> {
    CACHE.get().as_ref().unwrap()
}

pub fn get(parent: DirEntryItem, name: &String) -> Option<DirEntryItem> {
    let key = DirEntry::make_key(Some(&parent), &name);

    if let Some(e) = cache().get(key) {
        if e.parent().is_none() {
            e.update_parent(Some(parent));
        }

        Some(e)
    } else {
        None
    }
}

pub fn init() {
    CACHE.call_once(|| Cache::<CacheKey, DirEntry>::new(256));
}
