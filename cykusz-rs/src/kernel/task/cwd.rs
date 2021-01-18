use crate::kernel::fs::filesystem::Filesystem;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct Cwd {
    pub dentry: Arc<crate::kernel::fs::dirent::DirEntry>,
    pub fs: Arc<dyn Filesystem>,
}

impl Cwd {
    pub fn new(dentry: Arc<crate::kernel::fs::dirent::DirEntry>) -> Cwd {
        Cwd {
            dentry: dentry.clone(),
            fs: dentry.inode().fs(),
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
