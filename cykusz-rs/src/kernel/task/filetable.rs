use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::{OpenFlags, SysDirEntry};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::sync::RwLock;

const FILE_NUM: usize = 256;

pub struct FileHandle {
    pub fd: usize,
    pub inode: Arc<dyn INode>,
    pub offset: AtomicUsize,
    pub flags: OpenFlags,
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

    pub fn getdents(&self, mut buf: &mut [u8]) -> Result<usize> {
        let mut offset = 0usize;

        let struct_len = core::mem::size_of::<SysDirEntry>();

        Ok(loop {
            let dentry = self.inode.dirent(self.offset.load(Ordering::SeqCst))?;

            if let Some(d) = &dentry {
                let mut sysd = SysDirEntry {
                    ino: d.inode.id()?,
                    off: offset,
                    reclen: 0,
                    typ: d.inode.ftype()?,
                    name: [],
                };

                sysd.reclen = struct_len + d.name.len();
                sysd.off = offset + sysd.reclen;

                if buf.len() < sysd.reclen {
                    break offset;
                }

                self.offset.fetch_add(1, Ordering::SeqCst);

                unsafe {
                    buf.as_mut_ptr()
                        .copy_from(&sysd as *const _ as *const u8, struct_len);

                    let sysd_ref = &mut *(buf.as_mut_ptr() as *mut SysDirEntry);
                    sysd_ref.name.as_mut_ptr().copy_from(d.name.as_ptr(), d.name.len());
                }

                offset += sysd.reclen;
                buf = &mut buf[sysd.reclen..];
            } else {
                break offset;
            }
        })
    }
}

pub struct FileTable {
    files: RwLock<Vec<Option<Arc<FileHandle>>>>,
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

    pub fn open_file(&self, inode: Arc<dyn INode>, flags: OpenFlags) -> Option<usize> {
        let mut files = self.files.write();

        let mk_handle = |fd: usize, inode: Arc<dyn INode>| {
            Some(Arc::new(FileHandle {
                fd,
                inode,
                offset: AtomicUsize::new(0),
                flags,
            }))
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

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        let files = self.files.read();

        if let Some(handle) = &files[fd] {
            return Some(handle.clone());
        }

        None
    }
}
