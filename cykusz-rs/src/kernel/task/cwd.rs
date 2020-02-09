use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;

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

        let mut comps = if !p.is_absolute() {
            self.pwd.split("/").collect::<Vec<&str>>()
        } else {
            Vec::<&str>::new()
        };

        for el in p.components() {
            if el == ".." && comps.len() > 0 {
                comps.remove(comps.len() - 1);
            } else {
                if el != ".." {
                    comps.push(el);
                }
            }
        }

        let mut pwd = String::from("/");

        pwd.push_str(
            comps
                .join("/")
                .trim_start_matches("/")
                .trim_end_matches("/"),
        );

        self.pwd = pwd;
    }
}
