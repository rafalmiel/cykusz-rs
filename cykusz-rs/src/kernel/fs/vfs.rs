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

#[derive(Copy, Clone, PartialEq)]
pub enum FileType {
    File = 0x1,
    Dir = 0x2,
    DevNode = 0x3,
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
