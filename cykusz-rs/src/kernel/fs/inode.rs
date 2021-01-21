use alloc::sync::Arc;

use syscall_defs::FileType;

use crate::kernel::device::Device;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::vfs::{DirEntIter, FsError, Metadata, Result};
use crate::kernel::syscall::sys::PollTable;

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

    fn lookup(
        &self,
        _parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        _name: &str,
    ) -> Result<Arc<super::dirent::DirEntry>> {
        Err(FsError::NotSupported)
    }

    fn mkdir(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }

    fn rmdir(&self, _name: &str) -> Result<()> {
        Err(FsError::NotSupported)
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }

    fn poll(&self, _poll_table: Option<&mut PollTable>) -> Result<bool> {
        Err(FsError::NotSupported)
    }

    fn fs(&self) -> Arc<dyn Filesystem> {
        unimplemented!()
    }

    fn create(
        &self,
        _parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        _name: &str,
    ) -> Result<Arc<crate::kernel::fs::dirent::DirEntry>> {
        Err(FsError::NotSupported)
    }

    fn close(&self) {}

    fn mknode(&self, _name: &str, _devid: usize) -> Result<Arc<dyn INode>> {
        return Err(FsError::NotSupported);
    }

    fn symlink(&self, _name: &str, _target: &str) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn truncate(&self) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn dir_ent(
        &self,
        _parent: Arc<crate::kernel::fs::dirent::DirEntry>,
        _idx: usize,
    ) -> Result<Option<Arc<crate::kernel::fs::dirent::DirEntry>>> {
        return Err(FsError::NotSupported);
    }

    fn dir_iter(
        &self,
        _parent: Arc<crate::kernel::fs::dirent::DirEntry>,
    ) -> Option<Arc<dyn DirEntIter>> {
        None
    }

    fn device(&self) -> Result<Arc<dyn Device>> {
        return Err(FsError::EntryNotFound);
    }
}
