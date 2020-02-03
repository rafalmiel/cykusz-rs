use alloc::sync::Arc;

use crate::kernel::fs::vfs::{FsError, Result};

pub trait INode: Send + Sync {
    fn id(&self) -> usize;
    fn lookup(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }
    fn mkdir(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }

    fn open(&self, _name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn close(&self) -> Result<()> {
        Err(FsError::NotSupported)
    }
}
