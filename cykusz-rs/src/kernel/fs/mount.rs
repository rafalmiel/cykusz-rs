use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use hashbrown::HashMap;
use spin::Once;

use crate::kernel::device::dev_t::DevId;
use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::ext2::{Ext2Filesystem, FsDevice};
use crate::kernel::fs::filesystem::{Filesystem, FilesystemKind};
use crate::kernel::fs::ramfs::RamFS;
use crate::kernel::fs::root_dentry;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sync::{LockApi, Mutex, MutexGuard};

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

struct MountedFS {
    fs: Weak<dyn Filesystem>,
    ref_count: usize,
}

impl MountedFS {
    fn new(fs: &Arc<dyn Filesystem>) -> MountedFS {
        MountedFS {
            fs: Arc::downgrade(fs),
            ref_count: 1,
        }
    }
}

struct MountsData {
    mounts: BTreeMap<MountKey, Mountpoint>,
    mounted_devs: HashMap<DevId, MountedFS>,
}

impl MountsData {
    fn new() -> MountsData {
        MountsData {
            mounts: BTreeMap::new(),
            mounted_devs: HashMap::new(),
        }
    }
}

struct Mounts {
    mounts: Mutex<MountsData>,
}

impl Mounts {
    fn new() -> Mounts {
        Mounts {
            mounts: Mutex::new(MountsData::new()),
        }
    }

    fn make_key(e: &DirEntryItem) -> MountKey {
        let (i, s) = e.cache_key();

        (i, s)
    }

    fn make_fs(dev: Option<Arc<dyn FsDevice>>, typ: FilesystemKind) -> Option<Arc<dyn Filesystem>> {
        match (typ, dev) {
            (FilesystemKind::Ext2FS, Some(dev)) => Ext2Filesystem::new(dev),
            (FilesystemKind::RamFS, v) => Some(RamFS::new(v)),
            _ => None,
        }
    }

    fn do_mount(
        &self,
        dir: DirEntryItem,
        mut mounts: MutexGuard<MountsData>,
        fs_factory: impl FnOnce(&MutexGuard<MountsData>) -> Arc<dyn Filesystem>,
    ) -> Result<()> {
        if dir.is_mountpoint() {
            return Err(FsError::EntryNotFound);
        }

        let key = Self::make_key(&dir);

        if mounts.mounts.contains_key(&key) {
            Err(FsError::EntryExists)
        } else {
            let fs = fs_factory(&mounts);

            let dev_id = fs.device().id();
            let root = fs.root_dentry();

            root.update_name(dir.read().name.clone());
            root.update_parent(dir.read().parent.clone());

            mounts.mounts.insert(
                key,
                Mountpoint {
                    fs: fs.clone(),
                    root_entry: root.clone(),
                    orig_entry: dir.clone(),
                },
            );
            if let Err(mut e) = mounts.mounted_devs.try_insert(dev_id, MountedFS::new(&fs)) {
                e.entry.get_mut().ref_count += 1;
            }

            dir.set_is_mountpont(true);

            Ok(())
        }
    }

    fn mark_mounted(&self, fs: Arc<dyn Filesystem>) {
        let mut mounts = self.mounts.lock();

        if let Err(mut e) = mounts
            .mounted_devs
            .try_insert(fs.device().id(), MountedFS::new(&fs))
        {
            e.entry.get_mut().ref_count += 1;
        }
    }

    fn mount_fs(&self, dir: DirEntryItem, fs: Arc<dyn Filesystem>) -> Result<()> {
        self.do_mount(dir, self.mounts.lock(), |_| fs)
    }

    fn mount(
        &self,
        dir: DirEntryItem,
        dev: Option<Arc<dyn FsDevice>>,
        typ: FilesystemKind,
    ) -> Result<()> {
        let mounts = self.mounts.lock();

        self.do_mount(dir, mounts, |mnts| {
            if let Some(d) = &dev {
                if let Some(fs) = mnts.mounted_devs.get(&d.id()) {
                    return fs.fs.upgrade().unwrap().clone();
                }
            }
            Self::make_fs(dev, typ).unwrap()
        })
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

        if let Some(m) = mounts.mounts.get(&key) {
            //println!("found mount: {:?}", key);
            Ok(m.clone())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn unmount_by_key(&self, key: &MountKey) -> Result<()> {
        let mut mounts = self.mounts.lock();
        //println!("umount: {:?}", key);

        if let Some(ent) = mounts.mounts.get_mut(&key).cloned() {
            let id = ent.fs.device().id();
            let ref_cnt = mounts.mounted_devs.get(&id).unwrap().ref_count;

            logln!(
                "umount {:?} ref_cnt: {} dev_id: {} mountpoint strong count: {}",
                key,
                ref_cnt,
                id,
                ent.root_entry.strong_count()
            );

            if ent.root_entry.strong_count() > 3 {
                // given currently held pointers, strong count of 3 means the DirEntry is unused
                return Err(FsError::Busy);
            }

            ent.orig_entry.set_is_mountpont(false);

            mounts.mounts.remove(&key);

            if ref_cnt == 1 {
                ent.fs.umount();

                mounts.mounted_devs.remove(&id);
            } else {
                mounts.mounted_devs.get_mut(&id).unwrap().ref_count -= 1;
            }

            Ok(())
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn sync_all(&self) {
        let mounts = self.mounts.lock();

        for (_k, m) in mounts.mounts.iter() {
            m.fs.sync();
        }
    }

    fn umount_all(&self) {
        let mounts = self.mounts.lock();

        let keys: Vec<MountKey> = mounts.mounts.iter().map(|e| e.0.clone()).collect();

        drop(mounts);

        for key in keys.iter() {
            if let Err(_e) = self.unmount_by_key(key) {
                logln!("Failed to unmount {:?}", key);
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

pub fn mark_mounted(dev: Arc<dyn Filesystem>) {
    mounts().mark_mounted(dev);
}

pub fn mount_fs(dir: DirEntryItem, fs: Arc<dyn Filesystem>) -> Result<()> {
    mounts().mount_fs(dir, fs)
}

pub fn mount(dir: DirEntryItem, dev: Option<Arc<dyn FsDevice>>, typ: FilesystemKind) -> Result<()> {
    mounts().mount(dir, dev, typ)
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
