use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;

pub struct Cwd {
    pub dentry: DirEntryItem,
    pub fs: Arc<dyn Filesystem>,
}

impl Cwd {
    pub fn new(dentry: DirEntryItem) -> Option<Cwd> {
        if let Some(fs) = dentry.inode().fs().upgrade() {
            Some(Cwd {
                dentry: dentry.clone(),
                fs,
            })
        } else {
            None
        }
    }

    pub fn pwd(&self) -> String {
        let mut stack = Vec::<String>::new();

        let mut e = Some(self.dentry.clone());

        while let Some(el) = e {
            stack.push(el.name());

            e = el.read().parent.clone();
        }

        let mut res = String::new();

        for (i, s) in stack.iter().rev().enumerate() {
            if i > 1 {
                res += "/";
            }
            res += s.as_str();
        }

        res
    }
}
