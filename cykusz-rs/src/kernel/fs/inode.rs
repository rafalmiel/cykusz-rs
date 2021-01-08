use alloc::sync::Arc;

use syscall_defs::FileType;

use crate::kernel::device::Device;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::vfs::{DirEntry, FsError, Metadata, Result};
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

    fn poll(&self, _poll_table: Option<&mut PollTable>) -> Result<bool> {
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

    fn mount(&self, _fs: Arc<dyn Filesystem>) -> Result<Arc<dyn Filesystem>> {
        return Err(FsError::NotSupported);
    }

    fn umount(&self) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn device(&self) -> Result<Arc<dyn Device>> {
        return Err(FsError::EntryNotFound);
    }
}
