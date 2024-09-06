use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use hashbrown::HashSet;
use spin::Once;
use uuid::Uuid;

use crate::kernel::block::mbr::Partition;
use crate::kernel::device;
use crate::kernel::device::dev_t::DevId;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::cache::{ArcWrap, Cacheable};
use crate::kernel::fs::ext2::{Ext2Filesystem, FsDevice};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{
    CachedAccess, CachedBlockDev, PageCacheItem, PageCacheItemArc, PageCacheItemWeak, PageCacheKey,
    RawAccess,
};
use crate::kernel::params::params;
use crate::kernel::sync::{IrqGuard, LockApi, Mutex, MutexGuard};
use crate::kernel::timer::{create_timer, Timer, TimerCallback};
use crate::kernel::utils::types::CeilDiv;

mod mbr;

pub trait BlockDev: Send + Sync {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize>;
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize>;

    fn init_uuid(&self) -> Option<Uuid> {
        None
    }
}

static BLK_DEVS: Mutex<BTreeMap<DevId, Arc<BlockDevice>>> = Mutex::new(BTreeMap::new());

pub fn register_blkdev(dev: Arc<BlockDevice>) -> device::Result<()> {
    let mut devs = BLK_DEVS.lock();

    register_device(dev.clone())?;

    devs.insert(dev.id, dev.clone());

    drop(devs);

    dev.init();

    println!(
        "[ BLOCK ] Registered block device {} uuid {:?}",
        dev.name(),
        dev.uuid()
    );

    Ok(())
}

pub fn get_blkdev_by_id(id: DevId) -> Option<Arc<BlockDevice>> {
    let devs = BLK_DEVS.lock();

    if let Some(d) = devs.get(&id) {
        Some(d.clone())
    } else {
        None
    }
}

pub fn get_blkdev_by_uuid(uuid: Uuid) -> Option<Arc<BlockDevice>> {
    let devs = BLK_DEVS.lock();

    for (_k, v) in devs.iter() {
        if let Some(uid) = v.uuid() {
            if uid == uuid {
                return Some(v.clone());
            }
        }
    }

    None
}

pub fn get_blkdev_by_name(name: &str) -> Option<Arc<BlockDevice>> {
    let devs = BLK_DEVS.lock();

    for (_k, v) in devs.iter() {
        if name == v.name() {
            return Some(v.clone());
        }
    }

    None
}

pub struct BlockDevice {
    id: DevId,
    name: String,
    dev: Arc<dyn BlockDev>,
    self_ref: Weak<BlockDevice>,
    dirty_inode_pages: Mutex<hashbrown::HashMap<PageCacheKey, PageCacheItemWeak>>,
    dirty_pages: Mutex<hashbrown::HashMap<PageCacheKey, PageCacheItemWeak>>,
    cleanup_timer: Arc<Timer>,
    sync_all_altive: AtomicBool,
    uuid: Once<Option<Uuid>>,
}

impl FsDevice for BlockDevice {
    fn as_cached_device(&self) -> Option<Arc<dyn CachedBlockDev>> {
        let cd = self.self_ref.upgrade()?;
        return Some(cd.clone());
    }
}

pub struct PartitionBlockDev {
    offset: usize, // offset in sectors
    size: usize,   // capacity in sectors
    dev: Arc<dyn BlockDev>,
    self_ref: Weak<PartitionBlockDev>,
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

    fn init_uuid(&self) -> Option<Uuid> {
        Ext2Filesystem::try_get_uuid(self.self_ref.upgrade().unwrap())
    }
}

impl PartitionBlockDev {
    pub fn new(offset: usize, size: usize, dev: Arc<dyn BlockDev>) -> Arc<PartitionBlockDev> {
        logln!("new part at offset {} size: {}", offset, size);
        Arc::new_cyclic(|me| PartitionBlockDev {
            offset,
            size,
            dev,
            self_ref: me.clone(),
        })
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
            dev: imp.clone(),
            self_ref: me.clone(),
            dirty_inode_pages: Mutex::new(hashbrown::HashMap::new()),
            dirty_pages: Mutex::new(hashbrown::HashMap::new()),
            cleanup_timer: create_timer(TimerCallback::new(me.clone(), BlockDevice::sync_all)),
            sync_all_altive: AtomicBool::new(false),
            uuid: Once::new(),
        })
    }

    fn init(&self) {
        self.uuid.call_once(|| self.dev.init_uuid());
    }

    pub fn uuid(&self) -> Option<Uuid> {
        *self.uuid.get().unwrap()
    }

    fn sync_cache(
        &self,
        cache: &mut MutexGuard<hashbrown::HashMap<PageCacheKey, PageCacheItemWeak>>,
    ) {
        for (_, a) in cache.iter() {
            if let Some(up) = a.upgrade() {
                //logln!("sync page to storage (offset: {})", up.offset());
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
    fn id(&self) -> DevId {
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

    fn notify_dirty(&self, page: &PageCacheItemArc) {
        //logln!("notify dirty page: {}", page.offset());
        let mut dirty = self.dirty_pages.lock();

        dirty.insert(page.cache_key(), ArcWrap::downgrade(page));

        if !self.cleanup_timer.enabled() && !self.is_sync_all_active() {
            self.cleanup_timer.start_with_timeout(10_000);
        }
    }

    fn notify_clean(&self, page: &PageCacheItem) {
        //logln!("notify clean page: {}", page.offset());
        if !self.is_sync_all_active() {
            let mut dirty = self.dirty_pages.lock();

            dirty.remove(&page.cache_key());

            if dirty.is_empty() && self.dirty_inode_pages.lock().is_empty() {
                self.cleanup_timer.disable();
            }
        }
    }

    fn sync_page(&self, page: &PageCacheItem) {
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
    fn notify_dirty_inode(&self, page: &PageCacheItemArc) {
        //logln!("notify dirty inode: {}", page.offset());
        let mut dirty = self.dirty_inode_pages.lock();

        dirty.insert(page.cache_key(), ArcWrap::downgrade(page));

        if !self.cleanup_timer.enabled() && !self.is_sync_all_active() {
            self.cleanup_timer.start_with_timeout(10_000);
        }
    }

    fn notify_clean_inode(&self, page: &PageCacheItem) {
        //logln!("notify clean inode: {}", page.offset());
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

        self.cleanup_timer.disable();

        {
            let mut inodes = self.dirty_inode_pages.lock();
            dbgln!(sync, "Syncing... inodes {}", inodes.len(),);
            self.sync_cache(&mut inodes);
        }
        {
            let mut pages = self.dirty_pages.lock();
            dbgln!(sync, "Syncing... pages {}", pages.len(),);
            self.sync_cache(&mut pages);
        }

        dbgln!(sync, "Syncing... finished");
    }

    fn id(&self) -> DevId {
        self.id
    }

    fn device(&self) -> Arc<dyn FsDevice> {
        self.self_ref.upgrade().unwrap().clone()
    }
}

pub fn sync_all() {
    let blks = BLK_DEVS.lock();

    for (_k, blk) in blks.iter() {
        blk.sync_all();
    }
}

fn register_partition(dev: &Arc<BlockDevice>, count: usize, offset: usize, part: &Partition) {
    use crate::alloc::string::ToString;

    let part_dev = PartitionBlockDev::new(
        offset + part.relative_sector(),
        part.total_sectors(),
        dev.clone(),
    );

    let blkdev = BlockDevice::new(dev.name() + "." + &count.to_string(), part_dev);

    if let Err(e) = register_blkdev(blkdev.clone()) {
        panic!("Failed to register blkdev {} {:?}", blkdev.name(), e);
    }
}

fn process_partition(
    dev: &Arc<BlockDevice>,
    count: &mut usize,
    offset: usize,
    ext_offset: usize,
    part: &Partition,
) {
    if part.system_id() == 0 {
        return;
    }

    if part.system_id() != 5 {
        // not an extended partition
        register_partition(dev, *count, offset, &part);

        *count += 1;
    } else {
        // extended partition
        process_dev(
            dev.clone(),
            count,
            ext_offset + part.relative_sector(),
            if ext_offset > 0 {
                ext_offset
            } else {
                offset + part.relative_sector()
            },
        );
    }
}

fn process_dev(dev: Arc<BlockDevice>, count: &mut usize, offset: usize, ext_offset: usize) {
    let mut mbr = mbr::Mbr::new();
    dev.read(offset, mbr.bytes_mut());

    if !mbr.is_valid() {
        println!("[ WARN ] Mbr invalid for disk {}", dev.name());
        return;
    }

    for p in 0..4 {
        if let Some(part) = mbr.partition(p) {
            process_partition(&dev, count, offset, ext_offset, &part);
        }
    }
}

fn disks_to_scan() -> Option<HashSet<String>> {
    params().get("disks").and_then(|d| {
        Some(HashSet::<String>::from_iter(
            d.split(",").map(|e| String::from(e)),
        ))
    })
}

struct DisksToScan {
    disks: Option<HashSet<String>>,
}

impl DisksToScan {
    fn new() -> DisksToScan {
        DisksToScan {
            disks: disks_to_scan(),
        }
    }

    fn is_enabled(&self, name: &String) -> bool {
        if let Some(disks) = &self.disks {
            return disks.contains(name);
        }

        return true;
    }
}

pub fn init() {
    let mut devs = Vec::<Arc<BlockDevice>>::new();

    let disks = DisksToScan::new();

    for (_, dev) in BLK_DEVS.lock().iter() {
        if !disks.is_enabled(&dev.name()) {
            continue;
        }
        devs.push(dev.clone());
    }

    for dev in devs.iter() {
        let mut count = 1usize;
        process_dev(dev.clone(), &mut count, 0, 0);
    }
}
