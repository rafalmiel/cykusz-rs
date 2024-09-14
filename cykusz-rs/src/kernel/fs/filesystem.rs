use alloc::sync::Arc;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::FsDevice;

pub enum FilesystemKind {
    RamFS = 1,
    Ext2FS = 2,
}

pub trait Filesystem: Send + Sync {
    fn root_dentry(&self) -> DirEntryItem {
        unimplemented!()
    }

    fn sync(&self) {}

    fn umount(&self) {}

    fn name(&self) -> &'static str;

    fn device(&self) -> Arc<dyn FsDevice> {
        unimplemented!()
    }
}
