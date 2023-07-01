use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;

use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::root_dentry;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::Mutex;

#[derive(Clone)]
pub struct Mountpoint {
    fs: Arc<dyn Filesystem>,
    root_entry: DirEntryItem,
    orig_entry: DirEntryItem,
}

type MountKey = (usize, String);

impl Mountpoint {
    pub fn root_entry(&self) -> DirEntryItem {
        self.root_entry.clone()
    }
}

struct Mounts {
    mounts: Mutex<BTreeMap<MountKey, Mountpoint>>,
}

impl Mounts {
    fn new() -> Mounts {
        Mounts {
            mounts: Mutex::new(BTreeMap::new()),
        }
    }

    fn make_key(e: &DirEntryItem) -> MountKey {
        let (i, s) = e.cache_key();

        (i, s)
    }

    fn mount(&self, dir: DirEntryItem, fs: Arc<dyn Filesystem>) -> Result<()> {
        if dir.is_mountpoint() {
            return Err(FsError::EntryNotFound);
        }

        let key = Self::make_key(&dir);
        //println!("mounting key: {:?}", key);

        let mut mounts = self.mounts.lock();

        if mounts.contains_key(&key) {
            Err(FsError::EntryExists)
        } else {
            let root = fs.root_dentry();

            root.update_name(dir.read().name.clone());
            root.update_parent(dir.read().parent.clone());

            mounts.insert(
                key,
                Mountpoint {
                    fs,
                    root_entry: root,
                    orig_entry: dir.clone(),
                },
            );

            dir.set_is_mountpont(true);

            Ok(())
        }
    }

    fn umount(&self, dir: DirEntryItem) -> Result<()> {
        let key = Self::make_key(&dir);

        self.unmount_by_key(&key)
    }

    fn find_mount(&self, dir: &DirEntryItem) -> Result<Mountpoint> {
        if !dir.is_mountpoint() {
            //println!("is not mountpoint");
            return Err(FsError::EntryNotFound);
        }

        let key = Self::make_key(dir);
        //println!("find mountpoint: {:?}", key);

        let mounts = self.mounts.lock();

        if let Some(m) = mounts.get(&key) {
            //println!("found mount: {:?}", key);
            Ok(m.clone())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn unmount_by_key(&self, key: &MountKey) -> Result<()> {
        let mut mounts = self.mounts.lock();
        //println!("umount: {:?}", key);

        if let Some(ent) = mounts.get(&key) {
            if Arc::strong_count(&ent.fs) > 1 {
                return Err(FsError::Busy);
            }

            ent.orig_entry.set_is_mountpont(false);

            ent.fs.umount();

            mounts.remove(&key);

            Ok(())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn sync_all(&self) {
        let mounts = self.mounts.lock();

        for (_k, m) in mounts.iter() {
            m.fs.sync();
        }
    }

    fn umount_all(&self) {
        let mounts = self.mounts.lock();

        let keys: Vec<MountKey> = mounts.iter().map(|e| e.0.clone()).collect();

        drop(mounts);

        for key in keys.iter() {
            if let Err(_e) = self.unmount_by_key(key) {
                println!("Failed to unmount {:?}", key);
            }
        }

        root_dentry()
            .unwrap()
            .inode()
            .fs()
            .unwrap()
            .upgrade()
            .unwrap()
            .umount();
    }
}

static MOUNTS: Once<Mounts> = Once::new();

fn mounts() -> &'static Mounts {
    MOUNTS.get().unwrap()
}

pub fn init() {
    MOUNTS.call_once(|| Mounts::new());
}

pub fn mount(dir: DirEntryItem, fs: Arc<dyn Filesystem>) -> Result<()> {
    mounts().mount(dir, fs)
}

pub fn umount(dir: DirEntryItem) -> Result<()> {
    mounts().umount(dir)
}

pub fn find_mount(dir: &DirEntryItem) -> Result<Mountpoint> {
    mounts().find_mount(dir)
}

pub fn umount_all() {
    mounts().umount_all();
}

pub fn sync_all() {
    mounts().sync_all();
}
