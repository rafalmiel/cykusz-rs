use alloc::sync::Arc;
use alloc::sync::Weak;

use downcast_rs::DowncastSync;

use syscall_defs::FileType;

use crate::kernel::device::Device;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemInt};
use crate::kernel::fs::pcache::CachedAccess;
use crate::kernel::fs::vfs::{DirEntIter, FsError, Metadata, Result};
use crate::kernel::syscall::sys::PollTable;

pub trait INode: Send + Sync + DowncastSync {
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
        _parent: crate::kernel::fs::dirent::DirEntryItem,
        _name: &str,
    ) -> Result<super::dirent::DirEntryItem> {
        Err(FsError::NotSupported)
    }

    fn mkdir(&self, _name: &str) -> Result<INodeItem> {
        Err(FsError::NotSupported)
    }

    fn rmdir(&self, _name: &str) -> Result<()> {
        Err(FsError::NotSupported)
    }

    fn unlink(&self, _name: &str) -> Result<()> {
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

    fn fs(&self) -> Option<Weak<dyn Filesystem>> {
        None
    }

    fn create(&self, _parent: DirEntryItem, _name: &str) -> Result<DirEntryItem> {
        Err(FsError::NotSupported)
    }

    fn close(&self) {}

    fn mknode(&self, _name: &str, _devid: usize) -> Result<INodeItem> {
        return Err(FsError::NotSupported);
    }

    fn symlink(&self, _name: &str, _target: &str) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn link(&self, _name: &str, _target: INodeItem) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn rename(&self, _old: DirEntryItem, _new_name: &str) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn truncate(&self, _size: usize) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn dir_ent(&self, _parent: DirEntryItem, _idx: usize) -> Result<Option<DirEntryItem>> {
        return Err(FsError::NotSupported);
    }

    fn dir_iter(&self, _parent: DirEntryItem) -> Option<Arc<dyn DirEntIter>> {
        None
    }

    fn device(&self) -> Result<Arc<dyn Device>> {
        return Err(FsError::EntryNotFound);
    }

    fn ref_update(&self, _new_ref: Weak<INodeItemInt>) {}

    fn as_cacheable(&self) -> Option<Arc<dyn CachedAccess>> {
        None
    }
}

impl_downcast!(sync INode);
