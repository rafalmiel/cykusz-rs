use alloc::sync::Arc;

use spin::Once;

use syscall_defs::OpenFlags;

use crate::kernel::device::{register_device_listener, Device, DeviceListener};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::mountfs::MNode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task;

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

pub fn lookup_by_path(path: Path, lookup_mode: LookupMode) -> Result<Arc<dyn INode>> {
    let mut inode = if path.is_absolute() {
        root_inode().clone()
    } else {
        current_task().get_cwd().expect("CWD inode invalid")
    };

    let len = path.components().count();

    for (idx, name) in path.components().enumerate() {
        match inode.lookup(name) {
            Ok(i) => inode = i.inode,
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
