use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;

pub trait Filesystem: Send + Sync {
    fn root_inode(&self) -> Arc<dyn INode>;
}
