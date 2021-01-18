use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::sync::{RwSpin, RwSpinReadGuard, RwSpinWriteGuard, Spin};
use alloc::collections::BTreeMap;
use spin::Once;

#[derive(Clone)]
pub struct DirEntryData {
    pub parent: Option<Arc<DirEntry>>,
    pub name: String,
    pub inode: Arc<dyn INode>,
    fs: Option<Arc<dyn Filesystem>>,
}

pub struct DirEntry {
    data: RwSpin<DirEntryData>,
    used: AtomicBool,
    mountpoint: AtomicBool,
}

impl DirEntry {
    pub fn new_root(inode: Arc<dyn INode>, name: String) -> Arc<DirEntry> {
        Arc::new(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: None,
                name,
                inode: inode.clone(),
                fs: Some(inode.fs()),
            }),
            used: AtomicBool::new(false),
            mountpoint: AtomicBool::new(false),
        })
    }
    pub fn new_root_no_fs(inode: Arc<dyn INode>, name: String) -> Arc<DirEntry> {
        Arc::new(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: None,
                name,
                inode: inode.clone(),
                fs: None,
            }),
            used: AtomicBool::new(false),
            mountpoint: AtomicBool::new(false),
        })
    }

    pub fn new(parent: Arc<DirEntry>, inode: Arc<dyn INode>, name: String) -> Arc<DirEntry> {
        Arc::new(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: Some(parent),
                name,
                inode: inode.clone(),
                fs: Some(inode.fs()),
            }),
            used: AtomicBool::new(false),
            mountpoint: AtomicBool::new(false),
        })
    }

    pub fn inode_wrap(inode: Arc<dyn INode>) -> Arc<DirEntry> {
        Arc::new(DirEntry {
            data: RwSpin::new(DirEntryData {
                parent: None,
                name: String::new(),
                inode: inode.clone(),
                fs: None,
            }),
            used: AtomicBool::new(false),
            mountpoint: AtomicBool::new(false),
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

    pub fn inode(&self) -> Arc<dyn INode> {
        self.data.read().inode.clone()
    }

    pub fn mark_used(&self) {
        self.used.store(true, Ordering::SeqCst);
    }

    pub fn mark_unused(&self) {
        self.used.store(false, Ordering::SeqCst);
    }

    pub fn is_used(&self) -> bool {
        self.used.load(Ordering::SeqCst)
    }

    pub fn cache_key(&self) -> (usize, String) {
        let data = self.data.read();
        if let Some(parent) = &data.parent {
            (
                parent.as_ref() as *const DirEntry as usize,
                data.name.clone(),
            )
        } else {
            (0, data.name.clone())
        }
    }

    pub fn update_inode(&self, inode: Arc<dyn INode>) {
        self.data.write().inode = inode;
    }

    pub fn update_parent(&self, parent: Option<Arc<DirEntry>>) {
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

    pub fn set_fs(&self, fs: Option<Arc<dyn Filesystem>>) {
        self.data.write().fs = fs;
    }
}

impl Clone for DirEntry {
    fn clone(&self) -> Self {
        DirEntry {
            data: RwSpin::new(self.data.read().clone()),
            used: AtomicBool::new(self.used.load(Ordering::SeqCst)),
            mountpoint: AtomicBool::new(self.mountpoint.load(Ordering::SeqCst)),
        }
    }
}

impl Drop for DirEntry {
    fn drop(&mut self) {
        if self.is_used() {
            self.mark_unused();

            cache().move_to_unused(self.clone());
        }
    }
}

pub struct DirEntryCacheData {
    unused: lru::LruCache<(usize, String), Arc<DirEntry>>,
    used: BTreeMap<(usize, String), Weak<DirEntry>>,
}

pub struct DirEntryCache {
    data: Spin<DirEntryCacheData>,
}

impl DirEntryCacheData {
    fn get_dirent(&mut self, current: Arc<DirEntry>, name: String) -> Option<Arc<DirEntry>> {
        let key = (current.as_ref() as *const DirEntry as usize, name);

        if let Some(e) = self.used.get(&key) {
            e.clone().upgrade()
        } else {
            if let Some(e) = self.unused.get(&key) {
                let entry = e.clone();

                let inode = entry.inode();

                entry.write().parent = Some(current);
                entry.write().fs = Some(inode.fs());

                drop(e);

                self.unused.pop(&key);

                entry.mark_used();

                self.used.insert(key, Arc::downgrade(&entry));

                Some(entry)
            } else {
                None
            }
        }
    }

    fn insert(&mut self, entry: &Arc<DirEntry>) {
        let key = entry.cache_key();

        self.used.insert(key, Arc::downgrade(entry));
    }

    fn move_to_unused(&mut self, ent: DirEntry) {
        let key = ent.cache_key();

        ent.data.write().parent = None;
        ent.data.write().fs = None;

        if let Some(_e) = self.used.remove(&key) {
            self.unused.put(key, Arc::new(ent));
        } else {
            panic!("move_to_unused missing entry");
        }
    }
}
impl DirEntryCache {
    pub fn new() -> DirEntryCache {
        DirEntryCache {
            data: Spin::new(DirEntryCacheData {
                unused: lru::LruCache::new(256),
                used: BTreeMap::new(),
            }),
        }
    }

    pub fn get_dirent(&self, current: Arc<DirEntry>, name: String) -> Option<Arc<DirEntry>> {
        self.data.lock().get_dirent(current, name)
    }

    pub fn insert(&self, entry: &Arc<DirEntry>) {
        self.data.lock().insert(entry);
    }

    fn move_to_unused(&self, ent: DirEntry) {
        self.data.lock().move_to_unused(ent);
    }
}

static CACHE: Once<DirEntryCache> = Once::new();

pub fn cache() -> &'static DirEntryCache {
    CACHE.get().as_ref().unwrap()
}

pub fn init() {
    CACHE.call_once(|| DirEntryCache::new());
}
