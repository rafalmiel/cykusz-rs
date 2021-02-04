use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::kernel::device;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::cache::{ArcWrap, Cacheable};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{
    CachedAccess, PageCacheKey, PageItem, PageItemStruct, PageItemWeak, RawAccess,
};
use crate::kernel::sync::{RwSpin, Spin};
use crate::kernel::timer::{create_timer, Timer, TimerCallback};
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

pub fn get_blkdev_by_id(id: usize) -> Option<Arc<dyn CachedAccess>> {
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
    dirty_pages: Spin<hashbrown::HashMap<PageCacheKey, PageItemWeak>>,
    cleanup_timer: Arc<Timer>,
}

pub struct PartitionBlockDev {
    offset: usize, // offset in sectors
    size: usize,   // capacity in sectors
    dev: Arc<dyn BlockDev>,
}

impl BlockDev for PartitionBlockDev {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        if sector >= self.size {
            return None;
        }

        let max_count = self.size - 1 - sector;

        let count = dest.len().ceil_div(512);

        if count > max_count {
            self.dev.read(self.offset + sector, &mut dest[..max_count])
        } else {
            self.dev.read(self.offset + sector, dest)
        }
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        if sector >= self.size {
            return None;
        }

        let max_count = self.size - 1 - sector;

        let count = buf.len().ceil_div(512);

        if count > max_count {
            self.dev.write(self.offset + sector, &buf[..max_count])
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
        Arc::new_cyclic(|me| BlockDevice {
            id: alloc_id(),
            name,
            dev: imp,
            self_ref: me.clone(),
            dirty_pages: Spin::new(hashbrown::HashMap::new()),
            cleanup_timer: create_timer(TimerCallback::new(me.clone(), BlockDevice::sync_all)),
        })
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

impl<T: RawAccess> BlockDev for T {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        self.read_direct(sector, dest)
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.write_direct(sector, buf)
    }
}

impl RawAccess for BlockDevice {
    fn read_direct(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        self.dev.read(sector, dest)
    }

    fn write_direct(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.dev.write(sector, buf)
    }
}

impl CachedAccess for BlockDevice {
    fn this(&self) -> Weak<dyn CachedAccess> {
        self.self_ref.clone()
    }

    fn notify_dirty(&self, page: &PageItem) {
        self.write_direct(page.offset() * 8, page.data());

        let mut dirty = self.dirty_pages.lock();

        dirty.insert(page.cache_key(), ArcWrap::downgrade(page));

        if !self.cleanup_timer.enabled() {
            self.cleanup_timer.start_with_timeout(10_000);
        }
    }

    fn sync_page(&self, page: &PageItemStruct) {
        self.write_direct(page.offset() * 8, page.data());

        let mut dirty = self.dirty_pages.lock();
        dirty.remove(&page.cache_key());
    }

    fn sync_all(&self) {
        let mut dirty = self.dirty_pages.lock();

        for (_, p) in dirty.iter() {
            if let Some(a) = p.upgrade() {
                self.write_direct(a.offset() * 8, a.data());
            }
        }

        dirty.clear();
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
