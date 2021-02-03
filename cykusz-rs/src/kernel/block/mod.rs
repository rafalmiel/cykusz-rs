use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use downcast_rs::DowncastSync;

use crate::kernel::device;
use crate::kernel::device::{alloc_id, register_device, Device};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{cache, CachedAccess, PageItemStruct};
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::sync::RwSpin;
use crate::kernel::utils::types::{Align, CeilDiv};

mod mbr;

pub trait BlockDev: DowncastSync {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize>;
    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize>;
}

impl_downcast!(sync BlockDev);

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

impl BlockDev for BlockDevice {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        self.dev.read(sector, dest)
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        self.dev.write(sector, buf)
    }
}

impl CachedAccess for Arc<dyn BlockDev> {
    fn read_cached(&self, mut sector: usize, dest: &mut [u8]) -> Option<usize> {
        let page_cache = cache();

        let dev = Arc::downgrade(&self.clone().into_any_arc());

        let mut dest_offset = 0;

        while dest_offset < dest.len() {
            let cache_offset = sector / 8;

            if let Some(page) =
                if let Some(page) = page_cache.get(PageItemStruct::make_key(&dev, cache_offset)) {
                    Some(page)
                } else {
                    let new_page = PageItemStruct::new(dev.clone(), cache_offset);

                    self.read(sector.align(8), new_page.data_mut());

                    Some(page_cache.make_item(new_page))
                }
            {
                use core::cmp::min;

                let page_offset = (sector % 8) * 512;
                let to_copy = min(PAGE_SIZE - page_offset, dest.len() - dest_offset);

                dest[dest_offset..dest_offset + to_copy]
                    .copy_from_slice(&page.data()[page_offset..page_offset + to_copy]);

                dest_offset += to_copy;
                sector = (sector + 8).align(8);
            } else {
                break;
            }
        }

        Some(dest_offset)
    }

    fn write_cached(&self, mut sector: usize, buf: &[u8]) -> Option<usize> {
        self.write(sector, buf);

        let page_cache = cache();

        let dev = Arc::downgrade(&self.clone().into_any_arc());

        let mut copied = 0;

        while copied < buf.len() {
            let cache_offset = sector / 8;

            if let Some(page) =
                if let Some(page) = page_cache.get(PageItemStruct::make_key(&dev, cache_offset)) {
                    Some(page)
                } else {
                    let new_page = PageItemStruct::new(dev.clone(), cache_offset);

                    self.read(sector.align(8), new_page.data_mut());

                    Some(page_cache.make_item(new_page))
                }
            {
                use core::cmp::min;

                let page_offset = (sector % 8) * 512;
                let to_copy = min(PAGE_SIZE - page_offset, buf.len() - copied);

                page.data_mut()[page_offset..page_offset + to_copy]
                    .copy_from_slice(&buf[copied..copied + to_copy]);

                copied += to_copy;
                sector = (sector + 8).align(8);
            } else {
                break;
            }
        }

        Some(copied)
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
