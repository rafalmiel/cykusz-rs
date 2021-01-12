use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::kernel::device;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::inode::INode;
use crate::kernel::sync::RwSpin;
use crate::kernel::utils::types::CeilDiv;

mod mbr;

pub trait BlockDev: Send + Sync {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize>;
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize>;
}

static BLK_DEVS: RwSpin<BTreeMap<usize, Arc<BlockDevice>>> = RwSpin::new(BTreeMap::new());

pub fn register_blkdev(dev: Arc<BlockDevice>) -> device::Result<()> {
    let mut devs = BLK_DEVS.write();

    register_device(dev.clone())?;

    println!("[ BLOCK ] Registered block device {}", dev.name());

    devs.insert(dev.id, dev);

    Ok(())
}

pub fn get_blkdev_by_id(id: usize) -> Option<Arc<dyn BlockDev>> {
    let devs = BLK_DEVS.read();

    if let Some(d) = devs.get(&id) {
        Some(d.clone())
    } else {
        None
    }
}

pub struct BlockDevice {
    id: usize,
    name: String,
    dev: Arc<dyn BlockDev>,
    self_ref: Weak<BlockDevice>,
}

pub struct PartitionBlockDev {
    offset: usize, // offset in sectors
    size: usize,   // capacity in sectors
    dev: Arc<dyn BlockDev>,
}

impl BlockDev for PartitionBlockDev {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        let count = dest.len().ceil_div(512);

        if sector + count > self.size {
            return None;
        } else {
            self.dev.read(self.offset + sector, dest)
        }
    }
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        let count = buf.len().ceil_div(512);

        if sector + count > self.size {
            return None;
        } else {
            self.dev.write(self.offset + sector, buf)
        }
    }
}

impl PartitionBlockDev {
    pub fn new(offset: usize, size: usize, dev: Arc<dyn BlockDev>) -> PartitionBlockDev {
        PartitionBlockDev { offset, size, dev }
    }
}

impl BlockDevice {
    pub fn new(name: String, imp: Arc<dyn BlockDev>) -> Arc<BlockDevice> {
        BlockDevice {
            id: alloc_id(),
            name,
            dev: imp,
            self_ref: Weak::new(),
        }
        .wrap()
    }

    fn wrap(self) -> Arc<BlockDevice> {
        let fs = Arc::new(self);
        let weak = Arc::downgrade(&fs);
        let ptr = Arc::into_raw(fs) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
        }
    }
}

impl INode for BlockDevice {}

impl Device for BlockDevice {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.self_ref.upgrade().unwrap().clone()
    }
}

impl BlockDev for BlockDevice {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        self.dev.read(sector, dest)
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.dev.write(sector, buf)
    }
}

pub fn init() {
    use crate::alloc::string::ToString;

    let mut mbr = mbr::Mbr::new();

    let mut devs = Vec::<Arc<BlockDevice>>::new();

    for (_, dev) in BLK_DEVS.read().iter() {
        devs.push(dev.clone());
    }

    for dev in devs.iter() {
        dev.read(0, mbr.bytes_mut());

        if mbr.is_valid() {
            for p in 0..4 {
                if let Some(part) = mbr.partition(p) {
                    if part.total_sectors() > 0 {
                        let part_dev = PartitionBlockDev::new(
                            part.relative_sector() as usize,
                            part.total_sectors() as usize,
                            dev.clone(),
                        );

                        let blkdev = BlockDevice::new(
                            dev.name() + "." + &(p + 1).to_string(),
                            Arc::new(part_dev),
                        );

                        if let Err(e) = register_blkdev(blkdev.clone()) {
                            panic!("Failed to register blkdev {} {:?}", blkdev.name(), e);
                        }
                    }
                }
            }
        }
    }
}
