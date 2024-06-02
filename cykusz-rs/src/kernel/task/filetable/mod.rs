use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use syscall_defs::poll::PollEventFlags;
use syscall_defs::{
    FDFlags, FileType, OpenFlags, SeekWhence, SysDirEntry, SyscallError, SyscallResult,
};

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{DirEntIter, FsError, Result};
use crate::kernel::sync::{LockApi, Mutex, RwMutex};
use crate::kernel::task::filetable::inode_wrap::INodeOpsWrap;

mod inode_wrap;

const FILE_NUM: usize = 256;

pub struct FileHandle {
    fd: usize,
    inode: Arc<INodeOpsWrap>,
    offset: AtomicUsize,
    flags: AtomicUsize,
    dir_iter: Mutex<(Option<Arc<dyn DirEntIter>>, Option<DirEntryItem>)>,
}

impl FileHandle {
    pub fn new(fd: usize, inode: DirEntryItem, flags: OpenFlags) -> FileHandle {
        FileHandle {
            fd,
            inode: INodeOpsWrap::new(inode),
            offset: AtomicUsize::new(0),
            flags: AtomicUsize::from(flags.bits()),
            dir_iter: Mutex::new((None, None)),
        }
    }

    pub fn get_dir_item(&self) -> DirEntryItem {
        self.inode.get_dir_item()
    }

    pub fn get_inode(&self) -> &Arc<INodeOpsWrap> {
        &self.inode
    }

    pub fn open(&self, flags: OpenFlags) -> Result<()> {
        self.inode.open(flags)
    }

    pub fn close(&self, flags: OpenFlags) {
        self.inode.close(flags)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let offset = self.offset.load(Ordering::SeqCst);

        let read = self.read_at(buf, offset)?;

        //logln2!("read into buf {:?}", buf);

        self.offset.fetch_add(read, Ordering::SeqCst);

        Ok(read)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        let offset = self.offset.load(Ordering::SeqCst);

        //println!("writing to inode handle");
        let wrote = self.write_at(buf, offset)?;

        self.offset.fetch_add(wrote, Ordering::SeqCst);

        Ok(wrote)
    }

    pub fn read_at(&self, buf: &mut [u8], offset: usize) -> Result<usize> {
        Ok(self.inode.read_at(offset, buf)?)
    }

    pub fn write_at(&self, buf: &[u8], offset: usize) -> Result<usize> {
        let inode = &self.inode;
        Ok(match inode.as_cacheable() {
            Some(cacheable) => {
                if let Some(w) = cacheable.write_cached(offset, buf) {
                    w
                } else {
                    return Err(FsError::NotSupported);
                }
            }
            None => inode.write_at(offset, buf)?,
        })
    }

    pub fn seek(&self, off: isize, whence: syscall_defs::SeekWhence) -> Result<usize> {
        let meta = self.inode.metadata().ok().ok_or(FsError::IsPipe)?;

        if meta.typ == FileType::File {
            match whence {
                SeekWhence::SeekSet => {
                    self.offset.store(off as usize, Ordering::SeqCst);
                }
                SeekWhence::SeekCur => {
                    let mut offset = self.offset.load(Ordering::SeqCst) as isize;

                    offset += off;

                    self.offset.store(offset as usize, Ordering::SeqCst);
                }
                SeekWhence::SeekEnd => {
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

    pub fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize> {
        self.inode.ioctl(cmd, arg)
    }

    pub fn poll(
        &self,
        poll_table: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        self.inode.poll(poll_table, flags)
    }

    fn get_dir_iter(&self) -> (Option<Arc<dyn DirEntIter>>, Option<DirEntryItem>) {
        let mut lock = self.dir_iter.lock();

        if self.offset.load(Ordering::SeqCst) == 0 && lock.0.is_none() {
            let i = self
                .inode
                .get_fs_dir_item()
                .inode()
                .dir_iter(self.inode.get_fs_dir_item());
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
        OpenFlags::from_bits_truncate(self.flags.load(Ordering::Relaxed))
    }

    pub fn add_flags(&self, flags: OpenFlags) {
        let mask = OpenFlags::set_fd_flags_mask();
        self.flags.store(
            (self.flags().bits() & !mask) | (flags.bits() & mask),
            Ordering::Relaxed,
        );
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
                        None => self
                            .inode
                            .get_fs_dir_item()
                            .inode()
                            .dir_ent(self.inode.get_fs_dir_item(), o)?,
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
                    name.as_mut()
                        .unwrap()
                        .as_mut_ptr()
                        .copy_from(d.name().as_ptr(), d.name().len());
                    name.as_mut()
                        .unwrap()
                        .as_mut_ptr()
                        .offset(d.name().len() as isize)
                        .write(0);
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

impl Drop for FileHandle {
    fn drop(&mut self) {
        self.inode.close(self.flags());
    }
}

pub struct FileDescriptor {
    handle: Arc<FileHandle>,
    flags: AtomicU64,
}

impl FileDescriptor {
    fn new(handle: Arc<FileHandle>, flags: FDFlags) -> FileDescriptor {
        FileDescriptor {
            handle,
            flags: AtomicU64::new(flags.bits()),
        }
    }

    fn handle(&self) -> Arc<FileHandle> {
        self.handle.clone()
    }

    pub(crate) fn fd_flags(&self) -> FDFlags {
        FDFlags::from_bits_truncate(self.flags.load(Ordering::Relaxed))
    }

    pub(crate) fn set_fd_flags(&self, flags: FDFlags) {
        self.flags.store(flags.bits(), Ordering::Relaxed);
    }
}

impl Clone for FileDescriptor {
    fn clone(&self) -> Self {
        FileDescriptor {
            handle: self.handle.clone(),
            flags: AtomicU64::new(self.flags.load(Ordering::Relaxed)),
        }
    }
}

pub struct FileTable {
    files: RwMutex<Vec<Option<FileDescriptor>>>,
}

impl Clone for FileTable {
    fn clone(&self) -> FileTable {
        let files = self.files.read().clone();

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

    pub fn debug(&self) {
        for (i, f) in (&*self.files.read()).iter().enumerate() {
            if let Some(f) = f {
                logln5!(
                    "[{}] fd: {} {} {:?}",
                    i,
                    f.handle.fd,
                    f.handle.inode.get_fs_dir_item().full_path(),
                    f.handle.flags()
                );
            }
        }
    }

    pub fn open_file(&self, dentry: DirEntryItem, flags: OpenFlags) -> Result<usize> {
        let mut files = self.files.write();

        let append = flags.contains(OpenFlags::APPEND);

        let size = dentry.inode().stat()?.st_size;

        logln4!("open with append: {}, size: {}", append, size);

        let mk_handle = |fd: usize, inode: DirEntryItem| {
            Some(FileDescriptor::new(
                Arc::new(FileHandle {
                    fd,
                    inode: INodeOpsWrap::new(inode.clone()),
                    offset: AtomicUsize::new(if append { size as usize } else { 0 }),
                    flags: AtomicUsize::new(flags.bits()),
                    dir_iter: Mutex::new((None, None)),
                }),
                FDFlags::from(flags),
            ))
        };

        if let Some((idx, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            let h = mk_handle(idx, dentry).ok_or(FsError::Busy)?;

            h.handle().open(flags)?;

            *f = Some(h);

            Ok(idx)
        } else if files.len() < FILE_NUM {
            let len = files.len();

            let h = mk_handle(len, dentry).ok_or(FsError::Busy)?;

            h.handle().open(flags)?;

            files.push(Some(h));

            Ok(len)
        } else {
            Err(FsError::Busy)
        }
    }

    pub fn close_file(&self, fd: usize) -> bool {
        let mut files = self.files.write();

        if let Some(Some(_)) = &files.get(fd) {
            logln4!("close_file {}", fd);
            // inode.close() called on FileHandle Drop
            files[fd] = None;
            return true;
        }

        false
    }

    pub fn close_on_exec(&self) {
        let mut files = self.files.write();

        files
            .iter_mut()
            .filter(|p| {
                if let Some(p) = p {
                    p.fd_flags().contains(FDFlags::FD_CLOEXEC)
                } else {
                    false
                }
            })
            .for_each(|f| {
                // inode.close() called on FileHandle Drop
                *f = None;
            });
    }

    pub fn close_all_files(&self) {
        let mut files = self.files.write();

        files.clear();
        files.shrink_to_fit();
    }

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        let files = self.files.read();

        Some((files.get(fd)?.clone())?.handle())
    }

    pub fn get_fd(&self, fd: usize) -> Option<FileDescriptor> {
        let files = self.files.read();

        files.get(fd)?.clone()
    }

    pub fn duplicate(&self, fd: usize, flags: FDFlags, min: usize) -> SyscallResult {
        let handle = self.get_handle(fd).ok_or(SyscallError::EBADFD)?;

        let mut files = self.files.write();

        if let Some((idx, f)) = files
            .iter_mut()
            .enumerate()
            .find(|e| e.0 >= min && e.1.is_none())
        {
            // inode.close() called on FileHandle Drop
            *f = Some(FileDescriptor::new(handle, flags));

            Ok(idx)
        } else if files.len() < FILE_NUM {
            let len = files.len();

            files.push(Some(FileDescriptor::new(handle, flags)));

            Ok(len)
        } else {
            Err(SyscallError::EINVAL)
        }
    }

    pub fn duplicate_at(&self, fd: usize, at: usize, flags: FDFlags) -> SyscallResult {
        if at >= FILE_NUM {
            return Err(SyscallError::EINVAL);
        }

        let handle = self.get_handle(fd).ok_or(SyscallError::EBADFD)?;

        let mut files = self.files.write();

        // inode.close() called on FileHandle Drop
        files[at] = Some(FileDescriptor::new(handle, flags));

        Ok(at)
    }
}
