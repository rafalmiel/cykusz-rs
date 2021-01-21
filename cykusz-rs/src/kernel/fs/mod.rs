use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Try;

use spin::Once;

use syscall_defs::{FileType, OpenFlags};

use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;

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

static ROOT_MOUNT: Once<Arc<dyn Filesystem>> = Once::new();
static ROOT_DENTRY: Once<Arc<dirent::DirEntry>> = Once::new();

pub fn root_dentry() -> &'static Arc<dirent::DirEntry> {
    ROOT_DENTRY.get().unwrap()
}

struct DevListener {}

impl DeviceListener for DevListener {
    fn device_added(&self, dev: Arc<dyn Device>) {
        if let Ok(dev_dir) = root_dentry().inode().lookup(root_dentry().clone(), "dev") {
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
    dirent::init();
    mount::init();

    ROOT_DENTRY.call_once(|| {
        let fs = ramfs::RamFS::new();

        ROOT_MOUNT.call_once(|| fs.clone());

        let root = fs.root_dentry();

        root.inode()
            .mkdir("dev")
            .expect("Failed to create /dev directory");
        root.inode()
            .mkdir("etc")
            .expect("Failed to create /etc directory");
        root.inode()
            .mkdir("home")
            .expect("Failed to create /home directory");
        root.inode()
            .mkdir("var")
            .expect("Failed to create /var directory");
        root.inode()
            .mkdir("tmp")
            .expect("Failed to create /tmp directory");

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
    real_path: bool,
    depth: usize,
) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
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
                let r = dirent::cache()
                    .get_dirent(cur.clone(), String::from(s))
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
                                    root_dentry().clone()
                                },
                                real_path,
                                depth + 1,
                            )?;

                            res = if !real_path {
                                crate::kernel::fs::dirent::DirEntry::new_no_cache(
                                    cur.clone(),
                                    new.inode(),
                                    String::from(s),
                                )
                            } else {
                                new
                            }
                        }

                        cur = res;
                    }
                    Err(e)
                        if e == FsError::EntryNotFound
                            && idx == len - 1
                            && lookup_mode == LookupMode::Create =>
                    {
                        let inode = cur.inode();
                        let new = inode.create(cur.clone(), s)?;

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

pub fn lookup_by_path(
    path: Path,
    lookup_mode: LookupMode,
) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
    let cur = if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().clone()
    };

    lookup_by_path_from(path, lookup_mode, cur, false, 0)
}

pub fn lookup_by_real_path(
    path: Path,
    lookup_mode: LookupMode,
) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
    let cur = if !path.is_absolute() {
        current_task().get_dent()
    } else {
        root_dentry().clone()
    };

    lookup_by_path_from(path, lookup_mode, cur, true, 0)
}
