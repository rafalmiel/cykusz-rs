use crate::kernel::fs::dirent::DirEntryItem;

pub trait Filesystem: Send + Sync {
    fn root_dentry(&self) -> DirEntryItem {
        unimplemented!()
    }

    fn sync(&self) {}

    fn umount(&self) {}

    fn name(&self) -> &'static str;
}
