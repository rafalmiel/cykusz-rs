use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::RwSpin;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Once;

#[derive(Clone)]
pub struct Mountpoint {
    fs: Arc<dyn Filesystem>,
    root_entry: Arc<DirEntry>,
    orig_entry: Arc<DirEntry>,
}

impl Mountpoint {
    pub fn root_entry(&self) -> Arc<DirEntry> {
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

    fn mount(&self, dir: Arc<DirEntry>, fs: Arc<dyn Filesystem>) -> Result<()> {
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

    fn umount(&self, dir: Arc<DirEntry>) -> Result<()> {
        let key = dir.cache_key();

        let mut mounts = self.mounts.write();

        if let Some(ent) = mounts.get(&key) {
            ent.orig_entry.set_is_mountpont(false);

            drop(ent);

            mounts.remove(&key);

            Ok(())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn find_mount(&self, dir: &Arc<DirEntry>) -> Result<Mountpoint> {
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
}

static MOUNTS: Once<Mounts> = Once::new();

fn mounts() -> &'static Mounts {
    MOUNTS.get().unwrap()
}

pub fn init() {
    MOUNTS.call_once(|| Mounts::new());
}

pub fn mount(dir: Arc<DirEntry>, fs: Arc<dyn Filesystem>) -> Result<()> {
    mounts().mount(dir, fs)
}

pub fn umount(dir: Arc<DirEntry>) -> Result<()> {
    mounts().umount(dir)
}

pub fn find_mount(dir: &Arc<DirEntry>) -> Result<Mountpoint> {
    mounts().find_mount(dir)
}
