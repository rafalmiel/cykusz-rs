use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use downcast_rs::DowncastSync;

use syscall_defs::poll::PollEventFlags;
use syscall_defs::{FileType, OpenFlags};

use crate::kernel::device::Device;
use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemInt};
use crate::kernel::fs::pcache::{CachedAccess, MappedAccess};
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{DirEntIter, FsError, Metadata, Result};
use crate::kernel::net::socket::SocketService;

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

    fn stat(&self) -> Result<syscall_defs::stat::Stat> {
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

    fn read_all(&self) -> Vec<u8> {
        let meta = self.metadata().unwrap();
        let mut data = Vec::<u8>::new();
        data.resize(meta.size, 0);
        self.read_at(0, data.as_mut_slice())
            .expect("fstab read failed");
        data
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }

    fn poll(
        &self,
        _poll_table: Option<&mut PollTable>,
        _flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        Err(FsError::NotSupported)
    }

    fn fs(&self) -> Option<Weak<dyn Filesystem>> {
        None
    }

    fn create(&self, _parent: DirEntryItem, _name: &str) -> Result<DirEntryItem> {
        Err(FsError::NotSupported)
    }

    fn open(&self, _flags: OpenFlags) -> Result<()> {
        Ok(())
    }

    fn close(&self, _flags: OpenFlags) {}

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

    fn chmod(&self, _mode: syscall_defs::stat::Mode) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn utime(&self, _times: &[syscall_defs::time::Timespec; 2]) -> Result<()> {
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

    fn ioctl(&self, _cmd: usize, _arg: usize) -> Result<usize> {
        return Err(FsError::NoTty);
    }

    fn sync(&self) -> Result<()> {
        return Err(FsError::NotSupported);
    }

    fn ref_update(&self, _new_ref: Weak<INodeItemInt>) {}

    fn as_cacheable(&self) -> Option<Arc<dyn CachedAccess>> {
        None
    }

    fn as_mappable(&self) -> Option<Arc<dyn MappedAccess>> {
        logln!("Calling default???");
        None
    }

    fn as_socket(&self) -> Option<Arc<dyn SocketService>> {
        logln5!("as_socket default impl!!!!!!!!!");
        None
    }

    fn debug(&self) {}
}

impl_downcast!(sync INode);
