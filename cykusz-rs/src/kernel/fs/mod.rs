use alloc::sync::Arc;

use spin::Once;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::mountfs::MNode;

pub mod devfs;
pub mod filesystem;
pub mod inode;
pub mod mountfs;
pub mod path;
pub mod ramfs;
pub mod stdio;
pub mod vfs;

static ROOT_INODE: Once<Arc<MNode>> = Once::new();

fn root_inode() -> &'static Arc<MNode> {
    ROOT_INODE.r#try().unwrap()
}

pub fn init() {
    ROOT_INODE.call_once(|| {
        let fs = ramfs::RamFS::new();

        let mount_fs = mountfs::MountFS::new(fs);

        let root = mount_fs.root_inode();

        root.mkdir("dev")
            .expect("Failed to create /dev directory")
            .mount(devfs::DevFS::new())
            .expect("Failed to mount DevFS filesystem");

        root
    });

    stdio::init();
}

pub fn lookup_by_path(path: &str) -> Option<Arc<dyn INode>> {
    let path = path::Path::new(path);

    if !path.is_absolute() {
        panic!("Absolute path not yet supprted");
    }

    let mut inode = root_inode().clone();

    for name in path.components() {
        if let Ok(res) = inode.lookup(name) {
            inode = res;
        } else {
            return None;
        }
    }

    Some(inode)
}
