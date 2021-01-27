use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Try;

use spin::Once;

use syscall_defs::{FileType, OpenFlags};

use crate::kernel::block::get_blkdev_by_id;
use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::INodeItem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::ramfs::RamFS;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;

pub mod cache;
pub mod devnode;
pub mod dirent;
pub mod ext2;
pub mod filesystem;
pub mod icache;
pub mod inode;
pub mod mount;
pub mod path;
pub mod ramfs;
pub mod stdio;
pub mod vfs;

static ROOT_MOUNT: Once<Arc<dyn Filesystem>> = Once::new();
static ROOT_DENTRY: Once<DirEntryItem> = Once::new();

pub fn root_dentry() -> Option<&'static DirEntryItem> {
    ROOT_DENTRY.get()
}

struct DevListener {
    devfs: Arc<RamFS>,
}

impl DevListener {
    fn root_dentry(&self) -> DirEntryItem {
        self.devfs.root_dentry()
    }

    fn dev_inode(&self) -> INodeItem {
        self.devfs.root_dentry().inode().clone()
    }
}

impl DeviceListener for DevListener {
    fn device_added(&self, dev: Arc<dyn Device>) {
        self.dev_inode()
            .mknode(dev.name().as_str(), dev.id())
            .expect("Failed to mknode for device");
    }
}

static DEV_LISTENER: Once<Arc<DevListener>> = Once::new();

fn dev_listener() -> &'static Arc<DevListener> {
    DEV_LISTENER.get().unwrap()
}

pub fn init() {
    icache::init();
    dirent::init();
    mount::init();

    DEV_LISTENER.call_once(|| {
        let dev = Arc::new(DevListener {
            devfs: RamFS::new(),
        });

        register_device_listener(dev.clone());

        dev
    });

    stdio::init();
}

pub fn mount_root() {
    let dev_ent = dev_listener().root_dentry();

    let boot = dev_listener()
        .dev_inode()
        .lookup(dev_ent.clone(), "disk1.1")
        .unwrap();
    let root = dev_listener()
        .dev_inode()
        .lookup(dev_ent, "disk1.2")
        .unwrap();

    let boot_dev = get_blkdev_by_id(boot.inode().device().unwrap().id()).unwrap();
    let root_dev = get_blkdev_by_id(root.inode().device().unwrap().id()).unwrap();

    let boot_fs = Ext2Filesystem::new(boot_dev).expect("Invalid ext2 fs");
    let root_fs = Ext2Filesystem::new(root_dev).expect("Invalid ext2 fs");

    ROOT_MOUNT.call_once(|| root_fs.clone());
    ROOT_DENTRY.call_once(|| root_fs.root_dentry());

    let boot_entry =
        lookup_by_path(Path::new("/boot"), LookupMode::None).expect("/boot dir not found");

    mount::mount(boot_entry, boot_fs).expect("/boot mount failed");

    let dev_entry =
        lookup_by_path(Path::new("/dev"), LookupMode::None).expect("/dev dir not found");

    mount::mount(dev_entry, dev_listener().devfs.clone()).expect("/dev mount faiiled");
}

#[derive(Copy, Clone, PartialEq)]
pub enum LookupMode {
    None,
    Create,
}

impl From<OpenFlags> for LookupMode {
    fn from(f: OpenFlags) -> Self {
        if f.contains(OpenFlags::CREAT) {
            LookupMode::Create
        } else {
            LookupMode::None
        }
    }
}

fn read_link(inode: &Arc<dyn INode>) -> Result<String> {
    let mut path = Vec::<u8>::new();
    path.resize(128, 0);

    let mut offset = 0;

    loop {
        offset += inode.read_at(offset, &mut path.as_mut_slice()[offset..])?;

        if offset == path.len() {
            path.resize(offset + 128, 0);
        } else {
            break;
        }
    }

    Ok(String::from(unsafe {
        core::str::from_utf8_unchecked(&path.as_slice()[..offset])
    }))
}

fn lookup_by_path_from(
    path: Path,
    lookup_mode: LookupMode,
    mut cur: DirEntryItem,
    real_path: bool,
    depth: usize,
) -> Result<DirEntryItem> {
    //println!("looking up {}", path.str());
    if depth > 40 {
        println!("[ WARN ] Lookup recursion limit exceeded");
        return Err(FsError::EntryNotFound);
    }
    let len = path.components().count();

    for (idx, name) in path.components().enumerate() {
        //println!("lookup component {}", name);
        match name {
            "." => {}
            ".." => loop {
                let current = cur.read();
                if let Some(parent) = current.parent.clone() {
                    drop(current);

                    cur = parent.clone();
                }
                if cur.is_valid() {
                    break;
                }
            },
            s => {
                let r = dirent::get(cur.clone(), &String::from(s))
                    .into_result()
                    .or_else(|_| {
                        let current = cur.read();

                        current.inode.lookup(cur.clone(), s)
                    });

                match r {
                    Ok(mut res) => {
                        //println!("found {:?}", res.inode().ftype()?);

                        if res.inode().ftype()? == FileType::Symlink
                            && (depth > 0 || !real_path || idx < len - 1)
                        {
                            let link = read_link(&res.inode())?;
                            //println!("its symlink! {}", link);

                            let path = Path::new(link.as_str());

                            let is_absolute = path.is_absolute();

                            let new = lookup_by_path_from(
                                path,
                                lookup_mode,
                                if !is_absolute {
                                    cur.clone()
                                } else {
                                    root_dentry().unwrap().clone()
                                },
                                real_path,
                                depth + 1,
                            )?;

                            res = new;
                        }

                        cur = res;
                    }
                    Err(e)
                        if e == FsError::EntryNotFound
                            && idx == len - 1
                            && lookup_mode == LookupMode::Create =>
                    {
                        let inode = cur.inode();
                        //println!("Creating file with parent {} {:?}", cur.name(), cur.cache_key());
                        let new = inode.create(cur, s)?;

                        cur = new;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
        if cur.is_mountpoint() {
            if let Ok(mp) = mount::find_mount(&cur) {
                cur = mp.root_entry();
            }
        }
    }

    Ok(cur)
}

pub fn lookup_by_path(path: Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
    if let Some(cur) = if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().cloned()
    } {
        lookup_by_path_from(path, lookup_mode, cur, false, 0)
    } else {
        return Err(FsError::NotSupported);
    }
}

pub fn lookup_by_real_path(path: Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
    if let Some(cur) = if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().cloned()
    } {
        lookup_by_path_from(path, lookup_mode, cur, true, 0)
    } else {
        return Err(FsError::NotSupported);
    }
}
