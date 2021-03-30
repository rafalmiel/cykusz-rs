use syscall_defs::{FileType, SyscallError};

use crate::kernel::fs::dirent::DirEntryItem;

#[derive(Debug, PartialEq)]
pub enum FsError {
    NotSupported,
    NotFile,
    IsDir,
    NotDir,
    EntryNotFound,
    EntryExists,
    InvalidParam,
    Busy,
    Interrupted,
}

impl From<FsError> for syscall_defs::SyscallError {
    fn from(e: FsError) -> Self {
        match e {
            FsError::NotSupported => SyscallError::EACCES,
            FsError::NotFile => SyscallError::ENOENT,
            FsError::IsDir => SyscallError::EISDIR,
            FsError::NotDir => SyscallError::ENOTDIR,
            FsError::EntryNotFound => SyscallError::ENOENT,
            FsError::EntryExists => SyscallError::EEXIST,
            FsError::InvalidParam => SyscallError::EINVAL,
            FsError::Busy => SyscallError::EBUSY,
            FsError::Interrupted => SyscallError::EINTR,
        }
    }
}

pub type Result<T> = core::result::Result<T, FsError>;

pub trait DirEntIter: Send + Sync {
    fn next(&self) -> Option<DirEntryItem>;
}

#[derive(Copy, Clone)]
pub struct Metadata {
    pub id: usize,
    pub typ: FileType,
}
