use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;

use syscall_defs::{FileType, OpenFlags};

use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::inode::INode;

use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;
use crate::kernel::sync::RwSpin;

pub mod devnode;
pub mod dirent;
pub mod ext2;
pub mod filesystem;
pub mod inode;
pub mod mount;
pub mod path;
pub mod ramfs;
pub mod stdio;
pub mod vfs;

static ROOT_MOUNT: Once<RwSpin<Arc<dyn Filesystem>>> = Once::new();
static ROOT_INODE: Once<Arc<dyn INode>> = Once::new();
static ROOT_DENTRY: Once<Arc<dirent::DirEntry>> = Once::new();

pub fn root_inode() -> &'static Arc<dyn INode> {
    ROOT_INODE.get().unwrap()
}

pub fn root_dentry() -> &'static Arc<dirent::DirEntry> {
    ROOT_DENTRY.get().unwrap()
}

struct DevListener {}

impl DeviceListener for DevListener {
    fn device_added(&self, dev: Arc<dyn Device>) {
        if let Ok(dev_dir) = root_inode().lookup(root_dentry().clone(), "dev") {
            dev_dir
                .inode()
                .mknode(dev.name().as_str(), dev.id())
                .expect("Failed to mknode for device");
        } else {
            panic!("Failed to mknode for device {}", dev.name());
        }
    }
}

static DEV_LISTENER: Once<Arc<DevListener>> = Once::new();

pub fn init() {
    mount::init();

    ROOT_INODE.call_once(|| {
        let fs = ramfs::RamFS::new();
        println!("RamFS created");

        ROOT_MOUNT.call_once(|| RwSpin::new(fs.clone()));

        ROOT_DENTRY.call_once(|| fs.root_dentry());

        let root = fs.root_inode();

        root.mkdir("dev").expect("Failed to create /dev directory");
        root.mkdir("etc").expect("Failed to create /etc directory");
        root.mkdir("home")
            .expect("Failed to create /home directory");
        root.mkdir("var").expect("Failed to create /var directory");
        root.mkdir("tmp").expect("Failed to create /tmp directory");

        root
    });

    DEV_LISTENER.call_once(|| {
        let dev = Arc::new(DevListener {});

        register_device_listener(dev.clone());

        dev
    });

    stdio::init();
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
    mut cur: Arc<crate::kernel::fs::dirent::DirEntry>,
) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
    let len = path.components().count();

    for (idx, name) in path.components().enumerate() {
        match name {
            "." => {}
            ".." => {
                let current = cur.read();
                if let Some(parent) = current.parent.clone() {
                    drop(current);

                    cur = parent.clone();
                }
            }
            s => {
                {
                    let current = cur.read();
                    let cache = current.cache.upgrade().unwrap().clone();
                    if let Some(f) = cache.get_dirent(cur.clone(), String::from(s)) {
                        drop(current);

                        cur = f;
                    } else {
                        let r = current.inode.lookup(cur.clone(), s);

                        match r {
                            Ok(mut res) => {
                                drop(current);

                                if res.inode().ftype()? == FileType::Symlink {
                                    let link = read_link(&res.inode())?;

                                    let path = Path::new(link.as_str());

                                    let is_absolute = path.is_absolute();

                                    let new = lookup_by_path_from(
                                        path,
                                        lookup_mode,
                                        if !is_absolute {
                                            cur.clone()
                                        } else {
                                            root_dentry().clone()
                                        },
                                    )?;

                                    res = crate::kernel::fs::dirent::DirEntry::new(
                                        cur.clone(),
                                        cur.read().cache.clone(),
                                        new.inode(),
                                        String::from(s),
                                    );
                                }

                                cache.insert(&res);

                                cur = res;
                            }
                            Err(e)
                                if e == FsError::EntryNotFound
                                    && idx == len - 1
                                    && lookup_mode == LookupMode::Create =>
                            {
                                let inode = cur.inode();
                                let new = inode.create(cur.clone(), s)?;

                                drop(current);

                                cache.insert(&new);

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
        }
    }

    Ok(cur)
}

pub fn lookup_by_path(
    path: Path,
    lookup_mode: LookupMode,
) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
    let cur = if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().clone()
    };

    lookup_by_path_from(path, lookup_mode, cur)
}
