use alloc::sync::Arc;
use alloc::sync::Weak;
use core::ops::Deref;

use spin::Once;

use crate::kernel::fs::cache::{Cache, CacheItem, Cacheable};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::inode::INode;

type ICacheKey = (usize, usize);
type ICache = Cache<ICacheKey, INodeItemStruct>;

pub type INodeItemInt = CacheItem<ICacheKey, INodeItemStruct>;
pub type INodeItem = Arc<INodeItemInt>;

#[derive(Clone)]
pub struct INodeItemStruct {
    inode: Arc<dyn INode>,
}

impl INodeItemStruct {
    pub fn from(inode: Arc<dyn INode>) -> INodeItemStruct {
        INodeItemStruct { inode }
    }

    pub fn as_impl<T: INode>(&self) -> &T {
        match self.inode.downcast_ref::<T>() {
            Some(e) => e,
            _ => panic!("invalid conversion"),
        }
    }

    pub fn as_arc<T: INode>(&self) -> Arc<T> {
        match self.inode.clone().downcast_arc::<T>() {
            Ok(e) => e,
            _ => panic!("invalid conversion"),
        }
    }

    pub fn make_key(fs: &Weak<dyn Filesystem>, id: usize) -> ICacheKey {
        (Weak::as_ptr(fs) as *const () as usize, id)
    }
}

impl Deref for INodeItemStruct {
    type Target = Arc<dyn INode>;

    fn deref(&self) -> &Self::Target {
        &self.inode
    }
}

impl Cacheable<ICacheKey> for INodeItemStruct {
    fn cache_key(&self) -> (usize, usize) {
        INodeItemStruct::make_key(&self.fs(), self.id().unwrap())
    }

    fn make_unused(&self, new_ref: &Weak<INodeItemInt>) {
        self.inode.ref_update(new_ref.clone());
    }
}

static ICACHE: Once<Arc<ICache>> = Once::new();

pub fn cache() -> &'static Arc<ICache> {
    ICACHE.get().unwrap()
}

pub fn init() {
    ICACHE.call_once(|| ICache::new(256));
}
