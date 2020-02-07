use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::sync::RwLock;
use alloc::vec::Vec;

const FILE_NUM: usize = 256;

pub struct FileHandle {
    pub fd: usize,
    pub inode: Arc<dyn INode>,
    pub offset: AtomicUsize,
}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        FileHandle {
            fd: self.fd,
            inode: self.inode.clone(),
            offset: AtomicUsize::new(self.offset.load(Ordering::SeqCst)),
        }
    }
}

impl FileHandle {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let read = self
            .inode
            .read_at(self.offset.load(Ordering::SeqCst), buf)?;

        self.offset.fetch_add(read, Ordering::SeqCst);

        Ok(read)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        let wrote = self
            .inode
            .write_at(self.offset.load(Ordering::SeqCst), buf)?;

        self.offset.fetch_add(wrote, Ordering::SeqCst);

        Ok(wrote)
    }
}

pub struct FileTable {
    files: RwLock<Vec<Option<FileHandle>>>,
}

impl Default for FileTable {
    fn default() -> FileTable {
        FileTable::new()
    }
}

impl FileTable {
    pub fn new() -> FileTable {
        let mut files = Vec::new();
        files.resize(FILE_NUM, None);

        FileTable {
            files: RwLock::new(files),
        }
    }

    pub fn open_file(&self, inode: Arc<dyn INode>) -> Option<usize> {
        let mut files = self.files.write();

        let mk_handle = |fd: usize, inode: Arc<dyn INode>| {
            Some(FileHandle {
                fd,
                inode,
                offset: AtomicUsize::new(0),
            })
        };

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            *f = mk_handle(idx, inode);

            return Some(idx);
        } else if files.len() < FILE_NUM {
            let len = files.len();
            files.push(mk_handle(len, inode));

            return Some(len);
        }

        None
    }

    pub fn close_file(&self, fd: usize) -> bool {
        let mut files = self.files.write();

        if files[fd].is_some() {
            files[fd] = None;
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
