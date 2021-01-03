use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::kernel::fs::inode::INode;
use crate::kernel::sync::RwSpin;

pub trait Device: Send + Sync {
    fn id(&self) -> usize;
    fn name(&self) -> String;
    fn inode(&self) -> Arc<dyn INode>;
}

static FREE_DEV_ID: AtomicUsize = AtomicUsize::new(1);

static DEVICES: RwSpin<BTreeMap<usize, Arc<dyn Device>>> = RwSpin::new(BTreeMap::new());
static DEVICE_LISTEMERS: RwSpin<Vec<Arc<dyn DeviceListener>>> = RwSpin::new(Vec::new());

#[derive(Debug)]
pub enum DevError {
    DeviceExists = 0x1,
    DeviceNotFound = 0x2,
}

pub trait DeviceListener: Send + Sync {
    fn device_added(&self, dev: Arc<dyn Device>);
}

pub type Result<T> = core::result::Result<T, DevError>;

pub fn alloc_id() -> usize {
    FREE_DEV_ID.fetch_add(1, Ordering::SeqCst) << 32
}

pub fn register_device(dev: Arc<dyn Device>) -> Result<()> {
    let devs = DEVICES.read();

    if devs.contains_key(&dev.id()) {
        Err(DevError::DeviceExists)
    } else {
        drop(devs);
        DEVICES.write().insert(dev.id(), dev.clone());

        let listeners = DEVICE_LISTEMERS.read();
        for l in listeners.iter() {
            l.device_added(dev.clone());
        }
        Ok(())
    }
}

pub fn register_device_listener(listener: Arc<dyn DeviceListener>) {
    let mut l = DEVICE_LISTEMERS.write();

    l.push(listener);
}

pub fn devices() -> &'static RwSpin<BTreeMap<usize, Arc<dyn Device>>> {
    &DEVICES
}
