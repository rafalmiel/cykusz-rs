use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;
use uuid::Uuid;

use syscall_defs::{FileType, OpenFlags};

use crate::kernel::block::{get_blkdev_by_id, get_blkdev_by_name, get_blkdev_by_uuid};
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
pub mod pcache;
pub mod pipe;
pub mod poll;
pub mod ramfs;
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

#[allow(dead_code)]
fn init_cdboot() {
    let rootfs = RamFS::new();

    ROOT_MOUNT.call_once(|| rootfs.clone());

    ROOT_DENTRY.call_once(|| rootfs.root_dentry());

    root_dentry().unwrap().inode().mkdir("dev").unwrap();

    crate::kernel::fs::mount::mount(
        lookup_by_real_path(&Path::new("/dev"), LookupMode::None).unwrap(),
        dev_listener().devfs.clone(),
    )
    .expect("mount failed");
}

pub fn init() {
    pcache::init();
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
}

fn mount_by_path(path: &str, fs: Arc<dyn Filesystem>) {
    let entry = lookup_by_path(&Path::new(path), LookupMode::None)
        .expect((path.to_string() + " dir not found").as_str());

    mount::mount(entry, fs).expect((path.to_string() + " mount faiiled").as_str());
}

pub fn mount_root() {
    let uuid_str = crate::kernel::params::get("root").expect("missing root kernel cmd param");
    let root_dev = if let Ok(uuid) = Uuid::parse_str(uuid_str.as_str()) {
        get_blkdev_by_uuid(uuid).expect("device with root uuid does not exists")
    } else {
        get_blkdev_by_name(uuid_str).expect("device with root name {} does not exists")
    };

    let root_fs = Ext2Filesystem::new(root_dev).expect("Invalid ext2 fs");

    ROOT_MOUNT.call_once(|| root_fs.clone());
    ROOT_DENTRY.call_once(|| root_fs.root_dentry());

    if let Ok(fstab) = lookup_by_path(&Path::new("/etc/fstab"), LookupMode::None) {
        let data = fstab.inode().read_all();

        if let Ok(content) = core::str::from_utf8(data.as_slice()) {
            for line in content.split("\n") {
                if let Some((uuid_str, path)) = line.split_once(' ') {
                    if let Some(dev) =
                        get_blkdev_by_uuid(Uuid::parse_str(uuid_str).expect("Invalid uuid"))
                    {
                        mount_by_path(path, Ext2Filesystem::new(dev).expect("not ext2 filesystem"));

                        logln!("mounted uuid: {} at path: {}", uuid_str, path);
                    }
                }
            }
        }
    }

    mount_by_path("/dev", dev_listener().devfs.clone());
}

pub fn mount_root_old() {
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
        lookup_by_path(&Path::new("/boot"), LookupMode::None).expect("/boot dir not found");

    mount::mount(boot_entry, boot_fs).expect("/boot mount failed");

    let dev_entry =
        lookup_by_path(&Path::new("/dev"), LookupMode::None).expect("/dev dir not found");

    mount::mount(dev_entry, dev_listener().devfs.clone()).expect("/dev mount faiiled");
}

#[derive(Copy, Clone, PartialEq, Debug)]
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

pub fn read_link(inode: &Arc<dyn INode>) -> Result<String> {
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
    path: &Path,
    lookup_mode: LookupMode,
    mut cur: DirEntryItem,
    get_symlink_entry: bool,
    depth: usize,
) -> Result<DirEntryItem> {
    //println!("looking up {}", path.str());
    if depth > 40 {
        println!("[ WARN ] Lookup recursion limit exceeded");
        return Err(FsError::EntryNotFound);
    }
    let len = path.components().count();

    for (idx, name) in path.components().enumerate() {
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
                let r = dirent::get(cur.clone(), &String::from(s));

                let r = if let Some(r) = r {
                    Ok(r)
                } else {
                    let current = cur.read();

                    current.inode.lookup(cur.clone(), s)
                };

                //     .or_else(|_| {
                //         let current = cur.read();

                //         Some(current.inode.lookup(cur.clone(), s)?)
                //     }).ok_or(FsError::EntryNotFound);

                match r {
                    Ok(mut res) => {
                        //println!("found {:?}", res.inode().ftype()?);

                        if res.inode().ftype()? == FileType::Symlink
                            && (depth > 0 || !get_symlink_entry || idx < len - 1)
                        {
                            let link = read_link(&res.inode())?;
                            //println!("its symlink! {}", link);

                            let path = Path::new(link.as_str());

                            let is_absolute = path.is_absolute();

                            let new = lookup_by_path_from(
                                &path,
                                lookup_mode,
                                if !is_absolute {
                                    cur.clone()
                                } else {
                                    root_dentry().unwrap().clone()
                                },
                                get_symlink_entry,
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
        //println!("is cur mountpoint? {}", cur.is_mountpoint());
        if cur.is_mountpoint() {
            if let Ok(mp) = mount::find_mount(&cur) {
                cur = mp.root_entry();
            //println!("is mountpoint");
            } else {
                panic!("No mountpoint?");
            }
        }
    }

    Ok(cur)
}

pub fn lookup_by_path_at(
    dir: DirEntryItem,
    path: &Path,
    lookup_mode: LookupMode,
    get_symlink_entry: bool,
) -> Result<DirEntryItem> {
    if let Some(cur) = if !path.is_absolute() {
        Some(dir)
    } else {
        root_dentry().cloned()
    } {
        lookup_by_path_from(path, lookup_mode, cur, get_symlink_entry, 0)
    } else {
        return Err(FsError::NotSupported);
    }
}

pub fn lookup_by_path(path: &Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
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

pub fn lookup_by_real_path(path: &Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
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
