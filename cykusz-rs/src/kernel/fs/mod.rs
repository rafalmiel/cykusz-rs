use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;
use uuid::Uuid;

use syscall_defs::stat::Mode;
use syscall_defs::{FileType, OpenFlags};

use crate::kernel::block::{get_blkdev_by_name, get_blkdev_by_uuid};
use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::{Filesystem, FilesystemKind};
use crate::kernel::fs::icache::INodeItem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::pcache::CachedBlockDev;
use crate::kernel::fs::ramfs::RamFS;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;

pub mod cache;
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
    devfs: Arc<dyn Filesystem>,
}

impl DevListener {
    fn root_dentry(&self) -> DirEntryItem {
        self.devfs.root_dentry()
    }

    fn dev_inode(&self) -> INodeItem {
        self.devfs.root_dentry().inode().clone()
    }
}

pub trait FsDevice: Device {
    fn as_cached_device(&self) -> Option<Arc<dyn CachedBlockDev>> {
        None
    }
}

impl DeviceListener for DevListener {
    fn device_added(&self, dev: Arc<dyn Device>) {
        self.dev_inode()
            .mknode(
                self.root_dentry(),
                dev.name().as_str(),
                Mode::IFCHR,
                dev.id(),
            )
            .expect("Failed to mknode for device");
    }
}

static DEV_LISTENER: Once<Arc<DevListener>> = Once::new();

fn dev_listener() -> &'static Arc<DevListener> {
    DEV_LISTENER.get().unwrap()
}

pub fn init() {
    pipe::init();
    pcache::init();
    icache::init();
    dirent::init();
    mount::init();

    DEV_LISTENER.call_once(|| {
        let dev = Arc::new(DevListener {
            devfs: RamFS::new(None),
        });

        register_device_listener(dev.clone());

        dev
    });
}

fn mount_by_path(path: &str, dev: Option<Arc<dyn FsDevice>>, typ: FilesystemKind) {
    let entry = lookup_by_path(&Path::new(path), LookupMode::None)
        .expect((path.to_string() + " dir not found").as_str());

    mount::mount(entry, dev, typ).expect((path.to_string() + " mount faiiled").as_str());
}

fn mount_fs_by_path(path: &str, fs: Arc<dyn Filesystem>) {
    let entry = lookup_by_path(&Path::new(path), LookupMode::None)
        .expect((path.to_string() + " dir not found").as_str());

    mount::mount_fs(entry, fs).expect((path.to_string() + " mount faiiled").as_str());
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

    mount::mark_mounted(root_fs.clone());

    if let Ok(fstab) = lookup_by_path(&Path::new("/etc/fstab"), LookupMode::None) {
        let data = fstab.inode().read_all().expect("/etc/fstab read failed");

        if let Ok(content) = core::str::from_utf8(data.as_slice()) {
            for line in content.split("\n") {
                if let Some((uuid_str, path)) = line.split_once(' ') {
                    if let Some(dev) =
                        get_blkdev_by_uuid(Uuid::parse_str(uuid_str).expect("Invalid uuid"))
                    {
                        mount_by_path(path, Some(dev), FilesystemKind::Ext2FS);

                        logln!("mounted uuid: {} at path: {}", uuid_str, path);
                    }
                }
            }
        }
    }

    mount_fs_by_path("/dev", dev_listener().devfs.clone());
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
        offset += inode.read_at(
            offset,
            &mut path.as_mut_slice()[offset..],
            OpenFlags::empty(),
        )?;

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
                    let current = cur.inode();

                    current.lookup(cur.clone(), s)
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
                        let new = inode.create(cur, s, FileType::File)?;

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
            match mount::find_mount(&cur) { Ok(mp) => {
                cur = mp.root_entry();
            //println!("is mountpoint");
            } _ => {
                panic!("No mountpoint?");
            }}
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
    match if !path.is_absolute() {
        Some(dir)
    } else {
        root_dentry().cloned()
    } { Some(cur) => {
        //dbgln!(getdir, "lookup {}", path.str());
        lookup_by_path_from(path, lookup_mode, cur, get_symlink_entry, 0)
    } _ => {
        return Err(FsError::NotSupported);
    }}
}

pub fn lookup_by_path(path: &Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
    match if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().cloned()
    } { Some(cur) => {
        lookup_by_path_from(path, lookup_mode, cur, false, 0)
    } _ => {
        return Err(FsError::NotSupported);
    }}
}

pub fn lookup_by_real_path(path: &Path, lookup_mode: LookupMode) -> Result<DirEntryItem> {
    match if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().cloned()
    } { Some(cur) => {
        lookup_by_path_from(path, lookup_mode, cur, true, 0)
    } _ => {
        return Err(FsError::NotSupported);
    }}
}
