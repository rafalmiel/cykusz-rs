use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel::device;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::cache::{ArcWrap, Cacheable};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{
    CachedAccess, CachedBlockDev, PageCacheKey, PageItem, PageItemInt, PageItemWeak, RawAccess,
};
use crate::kernel::sync::{IrqGuard, Mutex, MutexGuard};
use crate::kernel::timer::{create_timer, Timer, TimerCallback};
use crate::kernel::utils::types::CeilDiv;

mod mbr;

pub trait BlockDev: Send + Sync {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize>;
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize>;
}

static BLK_DEVS: Mutex<BTreeMap<usize, Arc<BlockDevice>>> = Mutex::new(BTreeMap::new());

pub fn register_blkdev(dev: Arc<BlockDevice>) -> device::Result<()> {
    let mut devs = BLK_DEVS.lock();

    register_device(dev.clone())?;

    println!("[ BLOCK ] Registered block device {}", dev.name());

    devs.insert(dev.id, dev);

    Ok(())
}

pub fn get_blkdev_by_id(id: usize) -> Option<Arc<dyn CachedBlockDev>> {
    let devs = BLK_DEVS.lock();

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
    dirty_inode_pages: Mutex<hashbrown::HashMap<PageCacheKey, PageItemWeak>>,
    dirty_pages: Mutex<hashbrown::HashMap<PageCacheKey, PageItemWeak>>,
    cleanup_timer: Arc<Timer>,
    sync_all_altive: AtomicBool,
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

struct SyncAllGuard<'a> {
    blk_dev: &'a BlockDevice,
}

impl<'a> SyncAllGuard<'a> {
    fn new(blk: &'a BlockDevice) -> SyncAllGuard<'a> {
        blk.set_sync_all_active(true);

        SyncAllGuard::<'a> { blk_dev: blk }
    }
}

impl<'a> Drop for SyncAllGuard<'a> {
    fn drop(&mut self) {
        self.blk_dev.set_sync_all_active(false);
    }
}

impl BlockDevice {
    pub fn new(name: String, imp: Arc<dyn BlockDev>) -> Arc<BlockDevice> {
        Arc::new_cyclic(|me| BlockDevice {
            id: alloc_id(),
            name,
            dev: imp,
            self_ref: me.clone(),
            dirty_inode_pages: Mutex::new(hashbrown::HashMap::new()),
            dirty_pages: Mutex::new(hashbrown::HashMap::new()),
            cleanup_timer: create_timer(TimerCallback::new(me.clone(), BlockDevice::sync_all)),
            sync_all_altive: AtomicBool::new(false),
        })
    }

    fn sync_cache(&self, cache: &mut MutexGuard<hashbrown::HashMap<PageCacheKey, PageItemWeak>>) {
        for (_, a) in cache.iter() {
            if let Some(up) = a.upgrade() {
                logln!("sync page to storage (offset: {})", up.offset());
                up.sync_to_storage(&up);
            }
        }
        cache.clear();
    }

    fn is_sync_all_active(&self) -> bool {
        self.sync_all_altive.load(Ordering::SeqCst)
    }

    fn set_sync_all_active(&self, active: bool) {
        self.sync_all_altive.store(active, Ordering::SeqCst);
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
        self.read_direct(sector * 512, dest)
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.write_direct(sector * 512, buf)
    }
}

impl RawAccess for BlockDevice {
    fn read_direct(&self, offset: usize, dest: &mut [u8]) -> Option<usize> {
        assert_eq!(offset % 512, 0);
        self.dev.read(offset / 512, dest)
    }

    fn write_direct(&self, offset: usize, buf: &[u8]) -> Option<usize> {
        assert_eq!(offset % 512, 0);
        self.dev.write(offset / 512, buf)
    }
}

impl CachedAccess for BlockDevice {
    fn this(&self) -> Weak<dyn CachedAccess> {
        self.self_ref.clone()
    }

    fn notify_dirty(&self, page: &PageItem) {
        logln!("notify dirty page: {}", page.offset());
        let mut dirty = self.dirty_pages.lock();

        dirty.insert(page.cache_key(), ArcWrap::downgrade(page));

        if !self.cleanup_timer.enabled() && !self.is_sync_all_active() {
            self.cleanup_timer.start_with_timeout(10_000);
        }
    }

    fn notify_clean(&self, page: &PageItemInt) {
        logln!("notify clean page: {}", page.offset());
        if !self.is_sync_all_active() {
            let mut dirty = self.dirty_pages.lock();

            dirty.remove(&page.cache_key());

            if dirty.is_empty() && self.dirty_inode_pages.lock().is_empty() {
                self.cleanup_timer.disable();
            }
        }
    }

    fn sync_page(&self, page: &PageItemInt) {
        page.sync_to_storage(page);

        let key = page.cache_key();

        let mut dirty = self.dirty_pages.lock();
        if dirty.contains_key(&key) {
            dirty.remove(&key);

            return;
        }

        drop(dirty);

        dirty = self.dirty_inode_pages.lock();
        dirty.remove(&key);
    }
}

impl CachedBlockDev for BlockDevice {
    fn notify_dirty_inode(&self, page: &PageItem) {
        logln!("notify dirty inode: {}", page.offset());
        let mut dirty = self.dirty_inode_pages.lock();

        dirty.insert(page.cache_key(), ArcWrap::downgrade(page));

        if !self.cleanup_timer.enabled() && !self.is_sync_all_active() {
            self.cleanup_timer.start_with_timeout(10_000);
        }
    }

    fn notify_clean_inode(&self, page: &PageItemInt) {
        logln!("notify clean inode: {}", page.offset());
        if !self.is_sync_all_active() {
            let mut dirty = self.dirty_inode_pages.lock();

            dirty.remove(&page.cache_key());

            if dirty.is_empty() && self.dirty_pages.lock().is_empty() {
                self.cleanup_timer.disable();
            }
        }
    }

    fn sync_all(&self) {
        let _irq = IrqGuard::new();
        let _guard = SyncAllGuard::new(self);

        //self.cleanup_timer.disable();

        {
            let mut inodes = self.dirty_inode_pages.lock();
            logln!("Syncing... inodes {}", inodes.len(),);
            self.sync_cache(&mut inodes);
        }
        {
            let mut pages = self.dirty_pages.lock();
            logln!("Syncing... pages {}", pages.len(),);
            self.sync_cache(&mut pages);
        }

        logln!("Syncing... finished");
    }
}

pub fn sync_all() {
    let blks = BLK_DEVS.lock();

    for (_k, blk) in blks.iter() {
        blk.sync_all();
    }
}

pub fn init() {
    use crate::alloc::string::ToString;

    let mut mbr = mbr::Mbr::new();

    let mut devs = Vec::<Arc<BlockDevice>>::new();

    for (_, dev) in BLK_DEVS.lock().iter() {
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
