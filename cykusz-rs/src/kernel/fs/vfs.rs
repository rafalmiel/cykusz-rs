use alloc::string::String;
use alloc::sync::Arc;

use syscall_defs::SyscallError;

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

#[derive(Copy, Clone, PartialEq)]
pub enum FileType {
    File = 0x1,
    Dir = 0x2,
    DevNode = 0x3,
}

pub struct DirEntry {
    pub name: String,
    pub inode: Arc<dyn INode>,
}

impl Default for FileType {
    fn default() -> FileType {
        FileType::File
    }
}

#[derive(Copy, Clone)]
pub struct Metadata {
    pub id: usize,
    pub typ: FileType,
}
