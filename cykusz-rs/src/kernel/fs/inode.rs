use alloc::sync::Arc;

use crate::kernel::fs::vfs::{Result, FsError};

pub trait INode: Send + Sync {
    fn id(&self) -> usize;
    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }
    fn mkdir(&self, name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }

    fn open(&self, name: &str) -> Result<Arc<dyn INode>> {
        Err(FsError::NotSupported)
    }
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn close(&self) -> Result<()> {
        Err(FsError::NotSupported)
    }
}
