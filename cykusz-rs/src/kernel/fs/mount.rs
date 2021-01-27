use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;

use downcast_rs::__alloc::vec::Vec;
use spin::Once;

use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::RwSpin;

#[derive(Clone)]
pub struct Mountpoint {
    fs: Arc<dyn Filesystem>,
    root_entry: DirEntryItem,
    orig_entry: DirEntryItem,
}

impl Mountpoint {
    pub fn root_entry(&self) -> DirEntryItem {
        self.root_entry.clone()
    }
}

struct Mounts {
    mounts: RwSpin<BTreeMap<(usize, String), Mountpoint>>,
}

impl Mounts {
    fn new() -> Mounts {
        Mounts {
            mounts: RwSpin::new(BTreeMap::new()),
        }
    }

    fn mount(&self, dir: DirEntryItem, fs: Arc<dyn Filesystem>) -> Result<()> {
        if dir.is_mountpoint() {
            return Err(FsError::EntryNotFound);
        }

        let key = dir.cache_key();

        let mut mounts = self.mounts.write();

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
        let key = dir.cache_key();

        self.unmount_by_key(&key)
    }

    fn find_mount(&self, dir: &DirEntryItem) -> Result<Mountpoint> {
        if !dir.is_mountpoint() {
            return Err(FsError::EntryNotFound);
        }

        let key = dir.cache_key();

        let mounts = self.mounts.read();

        if let Some(m) = mounts.get(&key) {
            Ok(m.clone())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn unmount_by_key(&self, key: &(usize, String)) -> Result<()> {
        let mut mounts = self.mounts.write();

        if let Some(ent) = mounts.get(&key) {
            ent.orig_entry.set_is_mountpont(false);

            if Arc::strong_count(&ent.fs) > 1 {
                return Err(FsError::Busy);
            } else {
                ent.fs.umount();

                drop(ent);

                mounts.remove(&key);
            }

            Ok(())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn umount_all(&self) {
        let mounts = self.mounts.write();

        let keys: Vec<(usize, String)> = mounts.iter().map(|e| e.0.clone()).collect();

        drop(mounts);

        for key in keys.iter() {
            if let Err(_e) = self.unmount_by_key(key) {
                println!("Failed to unmount {:?}", key);
            }
        }
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
