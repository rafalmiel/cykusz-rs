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
}

impl From<FsError> for syscall_defs::SyscallError {
    fn from(e: FsError) -> Self {
        match e {
            FsError::NotSupported => SyscallError::Access,
            FsError::NotFile => SyscallError::NoEnt,
            FsError::IsDir => SyscallError::IsDir,
            FsError::NotDir => SyscallError::NotDir,
            FsError::EntryNotFound => SyscallError::NoEnt,
            FsError::EntryExists => SyscallError::Exists,
            FsError::InvalidParam => SyscallError::Inval,
            FsError::Busy => SyscallError::Busy,
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
