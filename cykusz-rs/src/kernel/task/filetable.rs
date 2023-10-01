use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use syscall_defs::{FileType, OpenFlags, SysDirEntry, SyscallError, SyscallResult};

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::vfs::{DirEntIter, FsError, Result};
use crate::kernel::sync::{Mutex, RwMutex};

const FILE_NUM: usize = 256;

pub struct FileHandle {
    pub fd: usize,
    pub inode: DirEntryItem,
    pub offset: AtomicUsize,
    pub flags: AtomicUsize,
    pub dir_iter: Mutex<(Option<Arc<dyn DirEntIter>>, Option<DirEntryItem>)>,
    //#[allow(unused)]
    //fs: Option<Arc<dyn Filesystem>>,
}

impl FileHandle {
    pub fn new(fd: usize, inode: DirEntryItem, flags: OpenFlags) -> FileHandle {
        FileHandle {
            fd,
            inode: inode.clone(),
            offset: AtomicUsize::new(0),
            flags: AtomicUsize::from(flags.bits()),
            dir_iter: Mutex::new((None, None)),
            //fs: if let Some(fs) = inode.inode().fs() {
            //    fs.upgrade()
            //} else {
            //    None
            //},
        }
    }

    pub fn duplicate(&self, flags: OpenFlags) -> Result<Arc<FileHandle>> {
        let flags = self.flags() | flags;

        let new = Arc::new(FileHandle::new(self.fd, self.inode.clone(), flags));

        new.inode.inode().open(flags)?;

        Ok(new)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let offset = self.offset.load(Ordering::SeqCst);

        let read = self.read_at(buf, offset)?;

        logln2!("read into buf {:?}", buf);

        self.offset.fetch_add(read, Ordering::SeqCst);

        Ok(read)
    }

    pub fn read_all(&self) -> Result<Vec<u8>> {
        let mut res = Vec::<u8>::new();
        res.resize(1024, 0);

        let mut size = 0;

        while let Ok(r) = self.read(&mut res.as_mut_slice()[size..size + 1024]) {
            size += r;

            if r < 1024 {
                res.shrink_to_fit();
                return Ok(res);
            }

            res.resize(size + 1024, 0);
        }

        Err(FsError::NotSupported)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        let offset = self.offset.load(Ordering::SeqCst);

        //println!("writing to inode handle");
        let wrote = self.write_at(buf, offset)?;

        self.offset.fetch_add(wrote, Ordering::SeqCst);

        Ok(wrote)
    }

    pub fn read_at(&self, buf: &mut [u8], offset: usize) -> Result<usize> {
        Ok(self.inode.inode().read_at(offset, buf)?)
    }

    pub fn write_at(&self, buf: &[u8], offset: usize) -> Result<usize> {
        Ok(match self.inode.inode().as_cacheable() {
            Some(cacheable) => {
                if let Some(w) = cacheable.write_cached(offset, buf) {
                    w
                } else {
                    return Err(FsError::NotSupported);
                }
            }
            None => self.inode.inode().write_at(offset, buf)?,
        })
    }

    pub fn seek(&self, off: isize, whence: syscall_defs::SeekWhence) -> Result<usize> {
        let meta = self.inode.inode().metadata().ok().ok_or(FsError::IsPipe)?;

        if meta.typ == FileType::File {
            match whence {
                syscall_defs::SeekWhence::SeekSet => {
                    self.offset.store(off as usize, Ordering::SeqCst);
                }
                syscall_defs::SeekWhence::SeekCur => {
                    let mut offset = self.offset.load(Ordering::SeqCst) as isize;

                    offset += off;

                    self.offset.store(offset as usize, Ordering::SeqCst);
                }
                syscall_defs::SeekWhence::SeekEnd => {
                    let mut offset = meta.size as isize;

                    offset += off;

                    self.offset.store(offset as usize, Ordering::SeqCst);
                }
            }

            Ok(self.offset.load(Ordering::SeqCst))
        } else {
            Err(FsError::IsPipe)
        }
    }

    fn get_dir_iter(&self) -> (Option<Arc<dyn DirEntIter>>, Option<DirEntryItem>) {
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

    pub fn flags(&self) -> OpenFlags {
        OpenFlags::from_bits_truncate(self.flags.load(Ordering::SeqCst))
    }

    pub fn add_flags(&self, flags: OpenFlags) {
        self.flags.store(self.flags().bits() | flags.bits(), Ordering::SeqCst);
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

                sysd.reclen = (struct_len + d.name().len()) as u16;
                sysd.off = offset + sysd.reclen as usize;

                if buf.len() < sysd.reclen as usize {
                    self.dir_iter.lock().1 = Some(d.clone());

                    break offset;
                }

                unsafe {
                    buf.as_mut_ptr()
                        .copy_from(&sysd as *const _ as *const u8, struct_len);

                    let sysd_ref = buf.as_mut_ptr() as *mut SysDirEntry;
                    let name = core::ptr::addr_of_mut!((*sysd_ref).name);
                    name.as_mut().unwrap().as_mut_ptr()
                        .copy_from(d.name().as_ptr(), d.name().len());
                    name.as_mut().unwrap().as_mut_ptr().offset(d.name().len() as isize).write(0);
                }

                offset += sysd.reclen as usize;
                self.offset.fetch_add(1, Ordering::SeqCst);
                buf = &mut buf[sysd.reclen.into()..];
            } else {
                break offset;
            }
        })
    }
}

pub struct FileTable {
    files: RwMutex<Vec<Option<Arc<FileHandle>>>>,
}

impl Clone for FileTable {
    fn clone(&self) -> FileTable {
        let files = self.files.read().clone();

        for f in &files {
            if let Some(f) = f {
                f.inode.inode().open(f.flags()).expect("Open failed");
            }
        }

        FileTable {
            files: RwMutex::new(files),
        }
    }
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
            files: RwMutex::new(files),
        }
    }

    pub fn open_file(
        &self,
        dentry: DirEntryItem,
        mut flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        let mut files = self.files.write();

        flags.remove(OpenFlags::CREAT);
        flags.remove(OpenFlags::DIRECTORY);

        let mk_handle = |fd: usize, inode: DirEntryItem| {
            Some(Arc::new(FileHandle {
                fd,
                inode: inode.clone(),
                offset: AtomicUsize::new(0),
                flags: AtomicUsize::new(flags.bits()),
                dir_iter: Mutex::new((None, None)),
                //fs: if let Some(fs) = inode.inode().fs() {
                //    fs.upgrade()
                //} else {
                //    None
                //},
            }))
        };

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            if let Some(h) = mk_handle(idx, dentry) {
                h.inode.inode().open(flags)?;

                *f = Some(h);

                Ok(idx)
            } else {
                println!("[ WARN ] Failed to open file");

                Err(FsError::NotSupported)
            }
        } else if files.len() < FILE_NUM {
            let len = files.len();

            if let Some(h) = mk_handle(len, dentry) {
                h.inode.inode().open(flags)?;

                files.push(Some(h));

                Ok(len)
            } else {
                Err(FsError::Busy)
            }
        } else {
            Err(FsError::Busy)
        }
    }

    pub fn close_file(&self, fd: usize) -> bool {
        let mut files = self.files.write();

        if let Some(f) = &files[fd] {
            f.inode.inode().close(f.flags());
            files[fd] = None;
            return true;
        }

        false
    }

    pub fn close_on_exec(&self) {
        let mut files = self.files.write();

        for f in files.iter_mut() {
            if let Some(h) = f {
                if h.flags().contains(OpenFlags::CLOEXEC) {
                    h.inode.inode().close(h.flags());

                    *f = None;
                }
            }
        }
    }

    pub fn close_all_files(&self) {
        let mut files = self.files.write();

        for f in files.iter_mut() {
            if let Some(file) = f {
                file.inode.inode().close(file.flags());

                *f = None;
            }
        }
        files.clear();
        files.shrink_to_fit();
    }

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        let files = self.files.read();

        if let Some(handle) = &files[fd] {
            return Some(handle.clone());
        }

        None
    }

    pub fn duplicate(&self, fd: usize, flags: OpenFlags, min: usize) -> SyscallResult {
        let handle = self.get_handle(fd).ok_or(SyscallError::EINVAL)?;

        let mut files = self.files.write();

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.0 >= min && e.1.is_none()) {
            *f = Some(handle.duplicate(flags)?);

            Ok(idx)
        } else if files.len() < FILE_NUM {
            let len = files.len();

            files.push(Some(handle.duplicate(flags)?));

            Ok(len)
        } else {
            Err(SyscallError::EINVAL)
        }
    }

    pub fn duplicate_at(&self, fd: usize, at: usize, flags: OpenFlags) -> SyscallResult {
        if at >= FILE_NUM {
            return Err(SyscallError::EINVAL);
        }

        let handle = self.get_handle(fd).ok_or(SyscallError::EINVAL)?;

        let mut files = self.files.write();

        if files[at].is_none() {
            files[at] = Some(handle.duplicate(flags)?);

            Ok(0)
        } else {
            match handle.duplicate(flags) {
                Ok(handle) => {
                    let old = files[at].take().unwrap();

                    old.inode.inode().close(old.flags());

                    files[at] = Some(handle);

                    Ok(0)
                }
                Err(e) => Err(e)?,
            }
        }
    }
}
