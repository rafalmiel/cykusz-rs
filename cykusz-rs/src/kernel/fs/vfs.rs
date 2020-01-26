#[derive(Debug)]
pub enum FsError {
    NotSupported,
    NotFile,
    IsDir,
    NotDir,
    EntryNotFound,
    EntryExists,
    InvalidParam,
}

pub type Result<T> = core::result::Result<T, FsError>;
