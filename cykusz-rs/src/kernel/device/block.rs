use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};

use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::inode::INode;
use crate::kernel::mm::Frame;
use crate::kernel::sync::RwSpin;

pub trait BlockDev: Send + Sync {
    fn read(&self, sector: usize, count: usize, dest: &mut [u8]) -> Option<usize>;
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize>;
}

static BLK_DEVS: RwSpin<BTreeMap<usize, Arc<BlockDevice>>> = RwSpin::new(BTreeMap::new());

pub fn register_blkdev(dev: Arc<BlockDevice>) -> super::Result<()> {
    register_device(dev.clone())?;

    let mut devs = BLK_DEVS.write();

    devs.insert(dev.id, dev);

    Ok(())
}

pub struct BlockDevice {
    id: usize,
    name: String,
    dev: Arc<dyn BlockDev>,
    self_ref: Weak<BlockDevice>,
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
    fn read(&self, sector: usize, count: usize, dest: &mut [u8]) -> Option<usize> {
        self.dev.read(sector, count, dest)
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.dev.write(sector, buf)
    }
}

pub fn test_read() {
    for (_, dev) in BLK_DEVS.read().iter() {
        let buf = allocate_order(0).unwrap().address_mapped();

        if let Some(r) = dev.read(0, 1, unsafe {
            core::slice::from_raw_parts_mut(buf.0 as *mut u8, 0x1000)
        }) {
            println!(
                "[ AHCI TEST ] Read {} bytes, value at 256: 0x{:x}",
                r,
                unsafe { (buf + 256).read::<u64>() }
            );
        }

        deallocate_order(&Frame::new(buf.to_phys()), 0);
    }
}
