use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::kernel::fs::inode::INode;
use crate::kernel::sync::RwLock;

pub trait Device: Send + Sync {
    fn id(&self) -> usize;
    fn name(&self) -> String;
    fn inode(&self) -> Arc<dyn INode>;
}

static FREE_DEV_ID: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref DEVICES: RwLock<BTreeMap<usize, Arc<dyn Device>>> = RwLock::new(BTreeMap::new());
}

#[derive(Debug)]
pub enum DevError {
    DeviceExists = 0x1,
}

pub type Result<T> = core::result::Result<T, DevError>;

pub fn alloc_id() -> usize {
    FREE_DEV_ID.fetch_add(1, Ordering::SeqCst) << 32
}

pub fn register_device(dev: Arc<dyn Device>) -> Result<()> {
    let mut devs = DEVICES.write();

    if devs.contains_key(&dev.id()) {
        Err(DevError::DeviceExists)
    } else {
        devs.insert(dev.id(), dev);
        Ok(())
    }
}

pub fn devices() -> &'static RwLock<BTreeMap<usize, Arc<dyn Device>>> {
    &DEVICES
}
