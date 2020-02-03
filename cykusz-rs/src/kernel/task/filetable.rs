use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use crate::kernel::sync::RwLock;

const FILE_NUM: usize = 16;

pub struct FileHandle {
    pub fd: usize,
    pub inode: Arc<dyn INode>,
}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        FileHandle {
            fd: self.fd,
            inode: self.inode.clone(),
        }
    }
}

pub struct FileTable {
    files: RwLock<[Option<FileHandle>; FILE_NUM]>,
}

impl Default for FileTable {
    fn default() -> FileTable {
        FileTable::new()
    }
}

impl FileTable {
    pub const fn new() -> FileTable {
        FileTable {
            files: RwLock::new([
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ]),
        }
    }

    pub fn open_file(&self, inode: Arc<dyn INode>) -> bool {
        let mut files = self.files.write();

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            *f = Some(FileHandle {
                fd: idx,
                inode: inode,
            });

            return true;
        }

        false
    }

    pub fn get_handle(&self, fd: usize) -> Option<FileHandle> {
        let files = self.files.read();

        if let Some(handle) = &files[fd] {
            return Some(handle.clone());
        }

        None
    }
}
