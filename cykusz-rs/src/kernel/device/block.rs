use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::arch::raw::mm::VirtAddr;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::inode::INode;
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
    let size = 1024*1024*4;
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(size, 0xCC);

    let addr = VirtAddr(buf.as_ptr() as usize);

    for (_, dev) in BLK_DEVS.read().iter() {
        if let Some(r) = dev.write(0, buf.as_slice()) {
            println!("[ BLOCK ] Test write of {} bytes", r);
        }

        if let Some(r) = dev.read(0, size / 512, unsafe {
            core::slice::from_raw_parts_mut(addr.0 as *mut u8, size)
        }) {
            let mut fail = false;
            print!(
                "[ BLOCK ] Test read  of {} bytes: ",
                r,
            );
            for &b in unsafe {
                addr.as_slice::<u64>(size / 8).iter().step_by(0x1000)
            } {
                if b != 0xCCCCCCCCCCCCCCCC {
                    fail = true;
                    break;
                }
            }

            if fail {
                println!("FAIL");
            } else {
                println!("OK");
            }

        }
    }
}
