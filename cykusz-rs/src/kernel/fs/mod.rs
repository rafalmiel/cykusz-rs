use alloc::sync::Arc;

use spin::Once;

use syscall_defs::{FileType, OpenFlags};

use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::mountfs::MNode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;
use alloc::string::String;
use alloc::vec::Vec;

pub mod devnode;
pub mod ext2;
pub mod filesystem;
pub mod inode;
pub mod mountfs;
pub mod path;
pub mod ramfs;
pub mod stdio;
pub mod vfs;

static ROOT_INODE: Once<Arc<MNode>> = Once::new();

pub fn root_inode() -> &'static Arc<MNode> {
    ROOT_INODE.get().unwrap()
}

struct DevListener {}

impl DeviceListener for DevListener {
    fn device_added(&self, dev: Arc<dyn Device>) {
        if let Ok(dev_dir) = root_inode().lookup("dev") {
            dev_dir
                .inode
                .mknode(dev.name().as_str(), dev.id())
                .expect("Failed to mknode for device");
        } else {
            panic!("Failed to mknode for device {}", dev.name());
        }
    }
}

static DEV_LISTENER: Once<Arc<DevListener>> = Once::new();

pub fn init() {
    ROOT_INODE.call_once(|| {
        let fs = ramfs::RamFS::new();

        let mount_fs = mountfs::MountFS::new(fs);

        let root = mount_fs.root_inode();

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
    mut inode: Arc<dyn INode>,
) -> Result<Arc<dyn INode>> {
    let len = path.components().count();

    for (idx, name) in path.components().enumerate() {
        match inode.lookup(name) {
            Ok(i) => {
                if i.inode.ftype()? == FileType::Symlink {
                    let link = read_link(&i.inode)?;

                    inode =
                        lookup_by_path_from(Path::new(link.as_str()), lookup_mode, inode.clone())?;
                } else {
                    inode = i.inode
                }
            }
            Err(e)
                if e == FsError::EntryNotFound
                    && idx == len - 1
                    && lookup_mode == LookupMode::Create =>
            {
                inode = inode.create(name)?;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(inode)
}

pub fn lookup_by_path(path: Path, lookup_mode: LookupMode) -> Result<Arc<dyn INode>> {
    let mut pwd = current_task().get_pwd();

    pwd.apply_path(path.str());

    lookup_by_path_from(Path::new(pwd.0.as_str()), lookup_mode, root_inode().clone())
}
