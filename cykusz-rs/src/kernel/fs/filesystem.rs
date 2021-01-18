use alloc::sync::Arc;

pub trait Filesystem: Send + Sync {
    fn root_dentry(&self) -> Arc<super::dirent::DirEntry> {
        unimplemented!()
    }

    fn sync(&self) {}
}
