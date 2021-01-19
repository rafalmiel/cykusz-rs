use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::{OpenFlags, SysDirEntry};

use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::vfs::{DirEntIter, Result};
use crate::kernel::sync::{Mutex, RwSpin};

const FILE_NUM: usize = 256;

pub struct FileHandle {
    pub fd: usize,
    pub inode: Arc<DirEntry>,
    pub offset: AtomicUsize,
    pub flags: OpenFlags,
    pub dir_iter: Mutex<(Option<Arc<dyn DirEntIter>>, Option<Arc<DirEntry>>)>,
}

impl FileHandle {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let read = self
            .inode
            .inode()
            .read_at(self.offset.load(Ordering::SeqCst), buf)?;

        self.offset.fetch_add(read, Ordering::SeqCst);

        Ok(read)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        let wrote = self
            .inode
            .inode()
            .write_at(self.offset.load(Ordering::SeqCst), buf)?;

        self.offset.fetch_add(wrote, Ordering::SeqCst);

        Ok(wrote)
    }

    fn get_dir_iter(&self) -> (Option<Arc<dyn DirEntIter>>, Option<Arc<DirEntry>>) {
        let mut lock = self.dir_iter.lock();

        if self.offset.load(Ordering::SeqCst) == 0 && lock.0.is_none() {
            let i = self.inode.inode().dir_iter(self.inode.clone());
            lock.0 = i;
        };

        let mut ret = (None, None);

        if let (Some(l), _) = &*lock {
            ret.0 = Some(l.clone());
        }

        if let (_, Some(l)) = &*lock {
            ret.1 = Some(l.clone());
        }

        lock.1 = None;

        ret
    }

    pub fn get_dents(&self, mut buf: &mut [u8]) -> Result<usize> {
        let mut offset = 0usize;

        let struct_len = core::mem::size_of::<SysDirEntry>();

        let (iter, mut cached) = self.get_dir_iter();

        Ok(loop {
            let dentry = {
                if cached.is_some() {
                    let res = cached.clone();
                    cached = None;
                    res
                } else {
                    let o = self.offset.load(Ordering::SeqCst);
                    match &iter {
                        Some(i) => i.next(),
                        None => self.inode.inode().dir_ent(self.inode.clone(), o)?,
                    }
                }
            };

            if let Some(d) = &dentry {
                let mut sysd = SysDirEntry {
                    ino: d.inode().id()?,
                    off: offset,
                    reclen: 0,
                    typ: d.inode().ftype()?,
                    name: [],
                };

                sysd.reclen = struct_len + d.name().len();
                sysd.off = offset + sysd.reclen;

                if buf.len() < sysd.reclen {
                    self.dir_iter.lock().1 = Some(d.clone());

                    break offset;
                }

                unsafe {
                    buf.as_mut_ptr()
                        .copy_from(&sysd as *const _ as *const u8, struct_len);

                    let sysd_ref = &mut *(buf.as_mut_ptr() as *mut SysDirEntry);
                    sysd_ref
                        .name
                        .as_mut_ptr()
                        .copy_from(d.name().as_ptr(), d.name().len());
                }

                offset += sysd.reclen;
                self.offset.fetch_add(1, Ordering::SeqCst);
                buf = &mut buf[sysd.reclen..];
            } else {
                break offset;
            }
        })
    }
}

pub struct FileTable {
    files: RwSpin<Vec<Option<Arc<FileHandle>>>>,
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
            files: RwSpin::new(files),
        }
    }

    pub fn open_file(&self, dentry: Arc<DirEntry>, flags: OpenFlags) -> Option<usize> {
        let mut files = self.files.write();

        let mk_handle = |fd: usize, inode: Arc<DirEntry>| {
            inode.ref_fs();

            Some(Arc::new(FileHandle {
                fd,
                inode,
                offset: AtomicUsize::new(0),
                flags,
                dir_iter: Mutex::new((None, None)),
                //fs: inode.inode().fs(),
            }))
        };

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            *f = mk_handle(idx, dentry);

            return Some(idx);
        } else if files.len() < FILE_NUM {
            let len = files.len();
            files.push(mk_handle(len, dentry));

            return Some(len);
        }

        None
    }

    pub fn close_file(&self, fd: usize) -> bool {
        let mut files = self.files.write();

        if let Some(f) = &files[fd] {
            f.inode.inode().close();
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
