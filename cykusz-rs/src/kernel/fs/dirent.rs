use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use spin::Once;

use crate::kernel::fs::cache::{ArcWrap, Cache, CacheItem, Cacheable, WeakWrap};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemStruct};
use crate::kernel::fs::inode::INode;
use crate::kernel::sync::{Mutex, MutexGuard};

type CacheKey = (usize, String);

impl Cacheable<CacheKey> for DirEntry {
    fn cache_key(&self) -> CacheKey {
        let data = self.read();
        Self::make_key(data.parent.as_ref(), &data.name)
    }

    fn notify_unused(&self, _new_ref: &Weak<CacheItem<CacheKey, DirEntry>>) {
        let mut data = self.write();
        logln!("mark unused: {}", data.name);
        data.fs_ref = None;
        data.parent = None;
    }

    fn notify_used(&self) {
        let mut data = self.write();

        logln!(
            "mark used: {}, parent: {}",
            data.name,
            data.parent.is_some()
        );

        data.fs_ref = if let Some(fs) = self.fs.get() {
            fs.upgrade()
        } else {
            None
        }
    }

    fn deallocate(&self, _me: &CacheItem<CacheKey, DirEntry>) {
        logln!("deallocate {}", _me.data.lock().name);
    }
}

pub type DirEntryItem = ArcWrap<CacheItem<CacheKey, DirEntry>>;
pub type DirEntryWeakItem = WeakWrap<CacheItem<CacheKey, DirEntry>>;

static CACHE_MARKER_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn new_cache_marker() -> usize {
    CACHE_MARKER_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub struct DirEntryData {
    pub parent: Option<DirEntryItem>,
    #[allow(unused)]
    fs_ref: Option<Arc<dyn Filesystem>>,
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
    data: Mutex<DirEntryData>,
    mountpoint: AtomicBool,
    fs: Once<Weak<dyn Filesystem>>,
    cache_marker: usize,
}

impl DirEntryItem {
    pub fn full_path(&self) -> String {
        let mut stack = Vec::<String>::new();

        let mut e = Some(self.clone());

        while let Some(el) = e {
            stack.push(el.name());

            e = el.read().parent.clone();
        }

        let mut res = String::new();

        for (i, s) in stack.iter().rev().enumerate() {
            if i > 1 {
                res += "/";
            }
            res += s.as_str();
        }

        res
    }
}

impl DirEntry {
    pub fn new_root(inode: INodeItem, name: String) -> DirEntryItem {
        cache().make_item_no_cache(DirEntry {
            data: Mutex::new(DirEntryData {
                parent: None,
                fs_ref: None,
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
                data: Mutex::new(DirEntryData {
                    parent: Some(parent.clone()),
                    fs_ref: None,
                    name,
                    inode: inode.clone(),
                }),
                mountpoint: AtomicBool::new(false),
                fs: if let Some(fs) = inode.fs() {
                    Once::initialized(fs)
                } else {
                    Once::new()
                },
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
            data: Mutex::new(DirEntryData {
                parent: Some(parent),
                fs_ref: None,
                name,
                inode: inode.clone(),
            }),
            mountpoint: AtomicBool::new(false),
            fs: if let Some(fs) = inode.fs() {
                Once::initialized(fs)
            } else {
                Once::new()
            },
            cache_marker: 0,
        })
    }

    pub fn inode_wrap(inode: Arc<dyn INode>) -> DirEntryItem {
        use crate::kernel::fs::icache;

        let cache = icache::cache();

        let item = cache.make_item_no_cache(INodeItemStruct::from(inode));

        DirEntry::inode_item_wrap(item)
    }

    pub fn inode_item_wrap(inode: INodeItem) -> DirEntryItem {
        cache().make_item_no_cache(DirEntry {
            data: Mutex::new(DirEntryData {
                parent: None,
                fs_ref: if let Some(fs) = inode.fs() {
                    if let Some(fs) = fs.upgrade() {
                        Some(fs)
                    } else {
                        None
                    }
                } else {
                    None
                },
                name: String::new(),
                inode: inode.clone(),
            }),
            mountpoint: AtomicBool::new(false),
            fs: if let Some(fs) = inode.fs() {
                Once::initialized(fs)
            } else {
                Once::new()
            },
            cache_marker: 0,
        })
    }

    pub fn read(&self) -> MutexGuard<DirEntryData> {
        self.data.lock()
    }

    pub fn write(&self) -> MutexGuard<DirEntryData> {
        self.data.lock()
    }

    pub fn name(&self) -> String {
        self.read().name.clone()
    }

    pub fn inode_id(&self) -> usize {
        self.read().inode.id().unwrap()
    }

    pub fn inode(&self) -> INodeItem {
        self.read().inode.clone()
    }

    pub fn make_key(parent: Option<&DirEntryItem>, name: &String) -> CacheKey {
        if let Some(p) = parent {
            (p.cache_marker, name.clone())
        } else {
            (0, name.clone())
        }
    }

    pub fn parent(&self) -> Option<DirEntryItem> {
        self.read().parent.clone()
    }

    pub fn update_inode(&self, inode: INodeItem) {
        self.write().inode = inode;
    }

    pub fn update_parent(&self, parent: Option<DirEntryItem>) {
        self.write().parent = parent;
    }

    pub fn update_name(&self, name: String) {
        self.write().name = name;
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
