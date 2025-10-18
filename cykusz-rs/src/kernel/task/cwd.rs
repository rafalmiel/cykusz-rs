use alloc::string::String;
use alloc::sync::Arc;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;

pub struct Cwd {
    pub dentry: DirEntryItem,
    pub fs: Arc<dyn Filesystem>,
}

impl Cwd {
    pub fn new(dentry: DirEntryItem) -> Option<Cwd> {
        match dentry.inode().fs().unwrap().upgrade() { Some(fs) => {
            Some(Cwd {
                dentry: dentry.clone(),
                fs,
            })
        } _ => {
            None
        }}
    }

    pub fn pwd(&self) -> String {
        self.dentry.full_path()
    }
}
