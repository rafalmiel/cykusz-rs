use alloc::sync::Arc;

use syscall_defs::FileType;

use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::vfs::{DirEntry, FsError, Metadata, Result};

pub trait INode: Send + Sync {
    fn id(&self) -> Result<usize> {
        Ok(self.metadata()?.id)
    }

    fn ftype(&self) -> Result<FileType> {
        Ok(self.metadata()?.typ)
    }

    fn metadata(&self) -> Result<Metadata> {
        Err(FsError::NotSupported)
    }

    fn lookup(&self, _name: &str) -> Result<DirEntry> {
        Err(FsError::NotSupported)
    }

    fn mkdir(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        unimplemented!()
    }

    fn create(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }

    fn close(&self) {}

    fn mknode(&self, _name: &str, _devid: usize) -> Result<Arc<dyn INode>> {
        return Err(FsError::NotSupported);
    }

    fn truncate(&self) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn dirent(&self, _idx: usize) -> Result<Option<DirEntry>> {
        return Err(FsError::NotSupported);
    }
}
