use alloc::string::String;
use alloc::sync::Arc;

use syscall_defs::{FileType, SyscallError};

use crate::kernel::fs::inode::INode;

#[derive(Debug, PartialEq)]
pub enum FsError {
    NotSupported,
    NotFile,
    IsDir,
    NotDir,
    EntryNotFound,
    EntryExists,
    InvalidParam,
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
        }
    }
}

pub type Result<T> = core::result::Result<T, FsError>;

pub struct DirEntry {
    pub name: String,
    pub inode: Arc<dyn INode>,
}

#[derive(Copy, Clone)]
pub struct Metadata {
    pub id: usize,
    pub typ: FileType,
}
