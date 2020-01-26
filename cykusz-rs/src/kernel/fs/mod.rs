use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use spin::Once;
use crate::kernel::fs::filesystem::Filesystem;

pub mod filesystem;
pub mod inode;
pub mod mountfs;
pub mod ramfs;
pub mod stdio;
pub mod vfs;

static ROOT_INODE: Once<Arc<dyn INode>> = Once::new();

fn root_inode() -> &'static Arc<dyn INode> {
    ROOT_INODE.r#try().unwrap()
}

pub fn init() {
    ROOT_INODE.call_once(|| {
        let fs = ramfs::RamFS::new();

        let mount_fs = mountfs::MountFS::new(fs);

        let root = mount_fs.root_inode();

        let dev = root.mkdir("dev").expect("Failed to create dev");

        let devfs = ramfs::RamFS::new();

        devfs.root_inode().mkdir("tty").expect("Dev2");

        dev.mount(devfs);

        root
    });

    stdio::init();

    if cfg!(not_exists) {
        if let Ok(dev) = root_inode().lookup("dev") {
            println!("Found dev!");

            if let Ok(dev2) = dev.lookup("tty") {
                println!("Found /dev/tty");
            }
        } else {
            println!("Dev not found!");
        }

    }
}
