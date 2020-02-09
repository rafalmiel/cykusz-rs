use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct Cwd {
    pub pwd: String,
    pub inode: Arc<dyn INode>,
}

impl Cwd {
    pub fn new(name: &str, inode: Arc<dyn INode>) -> Cwd {
        Cwd {
            pwd: String::from(name),
            inode,
        }
    }

    pub fn apply_path(&mut self, path: &str) {
        let p = Path::new(path);

        if p.is_absolute() {
            self.pwd = String::from(path);
            return;
        }

        let mut comps = self.pwd.split("/").collect::<Vec<&str>>();

        for el in p.components() {
            if el == ".." && comps.len() > 1 {
                comps.remove(comps.len() - 1);
            } else {
                comps.push(el);
            }
        }

        let mut pwd = String::from("/");

        use core::ops::Add;
        pwd.push_str(
            comps
                .join("/")
                .trim_start_matches("/")
                .trim_end_matches("/"),
        );

        self.pwd = pwd;
    }
}
