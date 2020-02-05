use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::sync::RwLock;

const FILE_NUM: usize = 16;

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

    pub fn open_file(&self, inode: Arc<dyn INode>) -> Option<usize> {
        let mut files = self.files.write();

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            *f = Some(FileHandle {
                fd: idx,
                inode: inode,
                offset: AtomicUsize::new(0),
            });

            return Some(idx);
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
