use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::ops::{Deref, DerefMut};

use intrusive_collections::LinkedList;

use syscall_defs::poll::PollEventFlags;
use syscall_defs::stat::Mode;
use syscall_defs::time::Timespec;
use syscall_defs::{FileType, OpenFlags};

use crate::arch::mm::PAGE_SIZE;
use crate::kernel::device::dev_t::DevId;
use crate::kernel::fs::cache::Cacheable;
use crate::kernel::fs::dirent::{DirEntry, DirEntryItem};
use crate::kernel::fs::ext2::dirent::{DirEntIter, SysDirEntIter};
use crate::kernel::fs::ext2::disk;
use crate::kernel::fs::ext2::idata::INodeData;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::{INodeItem, INodeItemStruct};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{
    CachedAccess, MappedAccess, PageCacheItem, PageCacheItemAdapter, PageCacheItemArc, RawAccess,
};
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::Metadata;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::mm::get_flags;
use crate::kernel::sync::{LockApi, Mutex, RwMutex, RwMutexReadGuard, RwMutexWriteGuard};
use crate::kernel::time::unix_timestamp;
use crate::kernel::utils::slice::ToBytes;

pub struct LockedExt2INode {
    node: RwMutex<Ext2INode>,
    fs: Weak<Ext2Filesystem>,
    self_ref: Weak<LockedExt2INode>,
    dirty_list: Mutex<LinkedList<PageCacheItemAdapter>>,
}

impl LockedExt2INode {
    pub fn new(fs: Weak<Ext2Filesystem>, id: usize) -> INodeItem {
        let cache = crate::kernel::fs::icache::cache();

        let fsg: Weak<dyn Filesystem> = fs.clone();

        if let Some(e) = cache.get(INodeItemStruct::make_key(&fsg, id)) {
            e
        } else {
            cache.make_item(INodeItemStruct::from(Arc::new_cyclic(|me| {
                LockedExt2INode {
                    node: RwMutex::new(Ext2INode::new(fs.clone(), id)),
                    fs,
                    self_ref: me.clone(),
                    dirty_list: Mutex::new(LinkedList::<PageCacheItemAdapter>::new(
                        PageCacheItemAdapter::new(),
                    )),
                }
            })))
        }
    }

    pub fn mk_dirent(&self, parent: DirEntryItem, de: &disk::dirent::DirEntry) -> DirEntryItem {
        let inode = self.ext2_fs().get_inode(de.inode() as usize);

        DirEntry::new(parent, inode, String::from(de.name()))
    }

    pub fn mk_inode(&self, typ: FileType) -> Result<INodeItem> {
        logln!("mk_inode: {:?}", typ);
        let fs = self.ext2_fs();

        let parent_id = self.id()?;

        if let Some(new) = fs.alloc_inode(parent_id) {
            let imp = new.as_impl::<LockedExt2INode>();

            let mut inner = imp.d_inode_writer();

            *inner = disk::inode::INode::default();

            inner.set_ftype(typ.into());
            inner.set_perm(0o644);
            inner.set_user_id(0);
            inner.set_group_id(0);

            let time = crate::kernel::time::unix_timestamp();
            inner.set_creation_time(time as u32);
            inner.set_last_modification(time as u32);
            inner.set_last_access(time as u32);

            drop(inner);

            let result: Result<()> = try {
                if typ == FileType::Dir {
                    let mut iter = DirEntIter::new_no_skip(new.as_ext2_inode_arc());
                    iter.add_dir_entry(imp, ".")?;
                    iter.add_dir_entry(&self, "..")?;
                }

                ()
            };

            if result.is_err() {
                fs.free_inode(imp);
            } else {
                return Ok(new);
            }
        }

        Err(FsError::NotSupported)
    }

    pub fn read(&self) -> RwMutexReadGuard<'_, Ext2INode> {
        self.node.read()
    }

    pub fn read_debug(&self, _id: usize) -> RwMutexReadGuard<'_, Ext2INode> {
        self.node.read()
    }

    pub fn d_inode_writer(&self) -> DINodeWriter<'_> {
        DINodeWriter {
            locked: self.write_debug(16),
            fs: self.fs.clone(),
            dirty: false,
        }
    }

    pub fn write(&self) -> RwMutexWriteGuard<'_, Ext2INode> {
        self.node.write()
    }

    pub fn write_debug(&self, _id: usize) -> RwMutexWriteGuard<'_, Ext2INode> {
        self.node.write()
    }

    pub fn fs(&self) -> Weak<Ext2Filesystem> {
        self.fs.clone()
    }

    pub fn ext2_fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
    }

    pub fn unref_hardlink(&self) {
        let inode = self.node.read();
        let id = inode.id;
        let hl_count = inode.d_inode().hl_count();

        drop(inode);

        if hl_count > 0 {
            let mut writer = self.d_inode_writer();

            logln_disabled!("dec_hl_count {}", hl_count - 1);
            writer.dec_hl_count();

            if hl_count == 1 {
                writer.set_deletion_time(crate::kernel::time::unix_timestamp() as u32);
            }

            logln_disabled!("unref {} hl: {}", id, hl_count - 1);
        }

        if hl_count == 1 {
            //It's 0 after decrement
            logln_disabled!("drop from cache {}", id);
            self.ext2_fs().drop_from_cache(id);
        }
    }

    fn self_ref(&self) -> Arc<LockedExt2INode> {
        self.self_ref.upgrade().unwrap()
    }

    fn update_at(&self, offset: usize, buf: &[u8], synced: bool) -> Result<usize> {
        if self.ftype()? != FileType::File && self.ftype()? != FileType::Symlink {
            return Err(FsError::NotFile);
        }

        let mut writer = INodeData::new_synced(self.self_ref(), offset, synced);

        Ok(writer.write(buf, false)?)
    }
}

impl Drop for LockedExt2INode {
    fn drop(&mut self) {
        let inode = self.node.read();
        dbgln!(ext2, "drop inode {}", inode.id);

        let hl_count = inode.d_inode.hl_count();
        let id = inode.id;

        drop(inode);

        dbgln!(ext2, "Dropping inode {} hl {}", id, hl_count);

        if hl_count == 0 {
            self.ext2_fs().free_inode(self)
        }
    }
}

pub struct DINodeWriter<'a> {
    locked: RwMutexWriteGuard<'a, Ext2INode>,
    fs: Weak<Ext2Filesystem>,
    dirty: bool,
}

impl<'a> DINodeWriter<'a> {
    pub fn id(&self) -> usize {
        self.locked.id
    }
}

impl<'a> Deref for DINodeWriter<'a> {
    type Target = disk::inode::INode;

    fn deref(&self) -> &Self::Target {
        &self.locked.d_inode
    }
}

impl<'a> DerefMut for DINodeWriter<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.locked.d_inode
    }
}

impl<'a> Drop for DINodeWriter<'a> {
    fn drop(&mut self) {
        if let Some(fs) = self.fs.upgrade() {
            if self.dirty {
                fs.group_descs()
                    .write_d_inode(self.locked.id, &self.locked.d_inode);
            }
        }
    }
}

pub struct Ext2INode {
    id: usize,
    d_inode: disk::inode::INode,
}

impl From<disk::inode::FileType> for syscall_defs::FileType {
    fn from(v: disk::inode::FileType) -> Self {
        match v {
            disk::inode::FileType::File => FileType::File,
            disk::inode::FileType::Symlink => FileType::Symlink,
            disk::inode::FileType::Dir => FileType::Dir,
            disk::inode::FileType::BlockDev => FileType::Block,
            disk::inode::FileType::CharDev => FileType::Char,
            disk::inode::FileType::Fifo => FileType::Fifo,
            disk::inode::FileType::Socket => FileType::Socket,
            disk::inode::FileType::Unknown => FileType::Unknown,
        }
    }
}

impl Ext2INode {
    pub fn new(fs: Weak<Ext2Filesystem>, id: usize) -> Ext2INode {
        let mut inode = Ext2INode {
            id,
            d_inode: disk::inode::INode::default(),
        };

        fs.upgrade()
            .unwrap()
            .group_descs()
            .read_d_inode(id, &mut inode.d_inode);

        inode
    }

    pub fn d_inode(&self) -> &disk::inode::INode {
        &self.d_inode
    }

    pub fn d_inode_mut(&mut self) -> &mut disk::inode::INode {
        &mut self.d_inode
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn ftype(&self) -> syscall_defs::FileType {
        self.d_inode.ftype().into()
    }

    fn free_s_ptr(
        &mut self,
        ptr: usize,
        fs: &Ext2Filesystem,
        current_offset: &mut usize,
        from_size: usize,
    ) -> bool {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        let mut res = true;

        for p in block.slice_mut() {
            if *p != 0 {
                if *current_offset >= from_size {
                    fs.group_descs().free_block_ptr(*p as usize);
                    self.d_inode.dec_sector_count(fs.sectors_per_block() as u32);

                    *p = 0;
                } else {
                    res = false;
                }
            }

            *current_offset += fs.block_size();
        }
        fs.write_block(ptr, block.slice().to_bytes());

        if res {
            fs.group_descs().free_block_ptr(ptr);
            self.d_inode.dec_sector_count(fs.sectors_per_block() as u32);
        }

        res
    }

    fn sync_s_ptr(&self, ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice() {
            if *p != 0 {
                fs.sync_block(*p as usize);
            } else {
                continue;
            }
        }

        fs.sync_block(ptr);
    }

    fn free_d_ptr(
        &mut self,
        ptr: usize,
        fs: &Ext2Filesystem,
        current_offset: &mut usize,
        from_size: usize,
    ) -> bool {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        let ptrs_per_block = block.len();

        let mut res = true;

        for p in block.slice_mut() {
            if *p != 0 {
                if self.free_s_ptr(*p as usize, fs, current_offset, from_size) {
                    *p = 0;
                } else {
                    res = false;
                }
            } else {
                *current_offset += ptrs_per_block * fs.block_size();
            }
        }
        fs.write_block(ptr, block.slice().to_bytes());

        if res {
            fs.group_descs().free_block_ptr(ptr);
            self.d_inode.dec_sector_count(fs.sectors_per_block() as u32);
        }

        res
    }

    fn sync_d_ptr(&self, ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice() {
            if *p != 0 {
                self.sync_s_ptr(*p as usize, fs);
            } else {
                continue;
            }
        }

        fs.sync_block(ptr);
    }

    fn free_t_ptr(
        &mut self,
        ptr: usize,
        fs: &Ext2Filesystem,
        current_offset: &mut usize,
        from_size: usize,
    ) -> bool {
        let mut block = fs.make_slice_buf_from::<u32>(ptr);

        let ptrs_per_block = block.len();

        let mut res = true;

        for p in block.slice_mut() {
            if *p != 0 {
                if self.free_d_ptr(*p as usize, fs, current_offset, from_size) {
                    *p = 0;
                } else {
                    res = false;
                }
            } else {
                *current_offset += ptrs_per_block * ptrs_per_block * fs.block_size();
            }
        }
        fs.write_block(ptr, block.slice().to_bytes());

        if res {
            fs.group_descs().free_block_ptr(ptr);
            self.d_inode.dec_sector_count(fs.sectors_per_block() as u32);
        }

        res
    }

    fn sync_t_ptr(&self, ptr: usize, fs: &Ext2Filesystem) {
        let block = fs.make_slice_buf_from::<u32>(ptr);

        for p in block.slice() {
            if *p != 0 {
                self.sync_d_ptr(*p as usize, fs);
            } else {
                continue;
            }
        }
        fs.sync_block(ptr);
    }

    fn free_blocks_from(&mut self, fs: &Ext2Filesystem, from_size: usize) {
        if self.ftype() == FileType::Symlink && self.d_inode.size_lower() <= 60 {
            return;
        }

        if ![FileType::File, FileType::Symlink, FileType::Dir].contains(&self.ftype()) {
            return;
        }

        dbgln!(
            ext2,
            "free inode blocks {} {:?} {:?}",
            self.id,
            self.d_inode,
            self.d_inode.block_ptrs()
        );
        let mut current_offset: usize = 0;

        for i in 0usize..15 {
            let ptr = self.d_inode.block_ptrs()[i] as usize;

            let ptrs_per_block = fs.block_size() / 4;

            if if i < 12 {
                let res = if ptr != 0 && current_offset >= from_size {
                    fs.group_descs().free_block_ptr(ptr);
                    self.d_inode.dec_sector_count(fs.sectors_per_block() as u32);

                    true
                } else {
                    false
                };

                current_offset += fs.block_size();

                res
            } else {
                match i {
                    12 => {
                        if ptr != 0 {
                            self.free_s_ptr(ptr, fs, &mut current_offset, from_size)
                        } else {
                            current_offset += ptrs_per_block * fs.block_size();

                            false
                        }
                    }
                    13 => {
                        if ptr != 0 {
                            self.free_d_ptr(ptr, fs, &mut current_offset, from_size)
                        } else {
                            current_offset += ptrs_per_block * ptrs_per_block * fs.block_size();

                            false
                        }
                    }
                    14 => {
                        if ptr != 0 {
                            self.free_t_ptr(ptr, fs, &mut current_offset, from_size)
                        } else {
                            current_offset +=
                                ptrs_per_block * ptrs_per_block * ptrs_per_block * fs.block_size();

                            false
                        }
                    }
                    _ => unreachable!(),
                }
            } {
                self.d_inode.block_ptrs_mut()[i] = 0;
            }
        }
        self.d_inode.set_size_lower(from_size as u32);
        logln!("freed inode blocks {} {:?}", self.id, self.d_inode);

        fs.group_descs().write_d_inode(self.id, self.d_inode());
    }

    pub fn free_blocks(&mut self, fs: &Ext2Filesystem) {
        self.free_blocks_from(fs, 0)
    }

    pub fn sync_blocks(&self, fs: &Ext2Filesystem) {
        if self.ftype() == FileType::Symlink && self.d_inode.size_lower() <= 60 {
            fs.group_descs().sync_d_inode(self.id);
            return;
        }

        logln!("sync inode blocks {} {:?}", self.id, self.d_inode);

        for i in 0usize..15 {
            let ptr = self.d_inode.block_ptrs()[i] as usize;

            if ptr == 0 {
                continue;
            }

            if i < 12 {
                fs.sync_block(ptr);
            } else {
                match i {
                    12 => {
                        self.sync_s_ptr(ptr, fs);
                    }
                    13 => {
                        self.sync_d_ptr(ptr, fs);
                    }
                    14 => {
                        self.sync_t_ptr(ptr, fs);
                    }
                    _ => unreachable!(),
                }
            }
        }

        fs.group_descs().sync_d_inode(self.id);
    }
}

impl RawAccess for LockedExt2INode {
    fn read_direct(&self, addr: usize, dest: &mut [u8]) -> Option<usize> {
        if let Ok(read) = self.read_at(addr, dest, OpenFlags::empty()) {
            Some(read)
        } else {
            None
        }
    }

    fn write_direct(&self, addr: usize, buf: &[u8]) -> Option<usize> {
        if let Ok(written) = self.update_at(addr, buf, false) {
            Some(written)
        } else {
            None
        }
    }

    fn write_direct_synced(&self, addr: usize, buf: &[u8]) -> Option<usize> {
        if let Ok(written) = self.update_at(addr, buf, true) {
            Some(written)
        } else {
            None
        }
    }
}

impl CachedAccess for LockedExt2INode {
    fn this(&self) -> Weak<dyn CachedAccess> {
        self.self_ref.clone()
    }

    fn notify_dirty(&self, page: &PageCacheItemArc) {
        page.link_to_list(&mut *self.dirty_list.lock());
        self.ext2_fs().dev().notify_dirty_inode(page);
    }

    fn notify_clean(&self, page: &PageCacheItem) {
        self.ext2_fs().dev().notify_clean_inode(page);
        if let Some(mut s) = self.dirty_list.try_lock() {
            //logln!("notify clean inode unlink");
            page.unlink_from_list(&mut *s);
        }
    }

    fn sync_page(&self, page: &PageCacheItem) {
        println!(
            "sync inode page flags {:?}",
            get_flags(page.page().to_virt())
        );
        if let Err(e) = self.update_at(page.offset() * PAGE_SIZE, page.data(), true) {
            panic!("Page {:?} sync failed {:?}", page.cache_key(), e);
        }
    }

    fn write_cached(&self, offset: usize, buf: &[u8]) -> Option<usize> {
        let cur_size = self.node.read().d_inode().size_lower() as usize;

        if cur_size < offset + buf.len() {
            if let Err(e) = self.truncate(offset + buf.len()) {
                println!("[ EXT2 ] truncate failed {:?}", e);
                return None;
            }
        }

        self.update_cached_synced(offset, buf, true)
    }
}

impl INode for LockedExt2INode {
    fn metadata(&self) -> Result<Metadata> {
        let inode = self.read();
        Ok(Metadata {
            id: inode.id,
            typ: inode.d_inode.ftype().into(),
            size: inode.d_inode.size_lower() as usize,
        })
    }

    fn stat(&self) -> Result<syscall_defs::stat::Stat> {
        let mut stat = syscall_defs::stat::Stat::default();

        let inode = self.read_debug(28);

        stat.st_ino = inode.id as u64;
        stat.st_dev = self.ext2_fs().dev().id() as u64;
        stat.st_nlink = inode.d_inode.hl_count() as u32;
        stat.st_blksize = self.ext2_fs().superblock().block_size() as u64;
        stat.st_blocks = inode.d_inode.sector_count() as u64;
        stat.st_size = inode.d_inode.size_lower() as i64;

        stat.st_atim =
            syscall_defs::time::Timespec::from_secs(inode.d_inode.last_access() as usize);
        stat.st_mtim =
            syscall_defs::time::Timespec::from_secs(inode.d_inode.last_modification() as usize);
        stat.st_ctim =
            syscall_defs::time::Timespec::from_secs(inode.d_inode.creation_time() as usize);

        let ftype = inode.ftype();
        stat.st_mode.insert(Mode::from(ftype));

        match ftype {
            FileType::Block | FileType::Char => {
                stat.st_rdev = inode.d_inode().get_rdevid();
            }
            _ => {
                stat.st_rdev = 0;
            }
        }

        stat.st_mode
            .insert(syscall_defs::stat::Mode::from_bits_truncate(
                inode.d_inode.perm() as u32,
            ));

        Ok(stat)
    }

    fn lookup(&self, parent: DirEntryItem, name: &str) -> Result<DirEntryItem> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        let mut iter = DirEntIter::new(self.self_ref());

        if let Some(e) = iter.find_map(|e| {
            if e.name() == name {
                Some(self.mk_dirent(parent.clone(), e))
            } else {
                None
            }
        }) {
            Ok(e)
        } else {
            Err(FsError::EntryNotFound)
        }
    }

    fn mkdir(&self, name: &str) -> Result<INodeItem> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if DirEntIter::new(self.self_ref())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        let new_inode = self.mk_inode(FileType::Dir)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref());

        if let Err(e) = iter.add_dir_entry(new_inode.as_ext2_inode(), name) {
            self.ext2_fs().free_inode(new_inode.as_ext2_inode());

            Err(e)
        } else {
            Ok(new_inode)
        }
    }

    fn rmdir(&self, name: &str) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        if [".", ".."].contains(&name) {
            return Err(FsError::NotSupported);
        }

        let _this = self.self_ref();

        if self.id()? == 2 {
            return Err(FsError::NotSupported);
        }

        // Check if dir is not empty
        if DirEntIter::new(self.self_ref())
            .find(|e| ![".", ".."].contains(&e.name()))
            .is_some()
        {
            return Err(FsError::NotSupported);
        }

        if let Some(parent) =
            if let Some(e) = DirEntIter::new(self.self_ref()).find(|e| e.name() == "..") {
                Some(self.ext2_fs().get_inode(e.inode() as usize))
            } else {
                None
            }
        {
            let mut iter = DirEntIter::new(self.self_ref());

            iter.remove_dir_entry(".")?;
            iter.remove_dir_entry("..")?;

            let mut iter = DirEntIter::new(parent.as_ext2_inode_arc());

            iter.remove_dir_entry(name)?;
        }

        Ok(())
    }

    fn unlink(&self, name: &str) -> Result<()> {
        logln_disabled!("unlink started");

        let fs = self.ext2_fs();
        let _lock = fs.dir_lock();

        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        if [".", ".."].contains(&name) {
            return Err(FsError::NotSupported);
        }

        let _this = self.self_ref();

        if let Some(target) = DirEntIter::new(self.self_ref()).find(|e| e.name() == name) {
            let inode = self.ext2_fs().get_inode(target.inode() as usize);

            if inode.ftype()? == FileType::Dir {
                return Err(FsError::NotFile);
            }
        }

        let mut iter = DirEntIter::new(self.self_ref());

        logln!("unlink: Remove dir entry: {}", name);
        let dir_iter = DirEntIter::new(self.self_ref());
        for e in dir_iter {
            logln!("{:?}", e);
        }
        iter.remove_dir_entry(name)?;

        let dir_iter = DirEntIter::new(self.self_ref());

        logln!("unlink: Removed!!! dir entry: {}", name);
        for e in dir_iter {
            logln!("{:?}", e);
        }

        Ok(())
    }

    fn read_at(&self, offset: usize, buf: &mut [u8], _flags: OpenFlags) -> Result<usize> {
        if self.ftype()? != FileType::File && self.ftype()? != FileType::Symlink {
            return Err(FsError::NotFile);
        }

        let mut reader = INodeData::new(self.self_ref(), offset);

        Ok(reader.read(buf))
    }

    fn write_at(&self, offset: usize, buf: &[u8], _flags: OpenFlags) -> Result<usize> {
        if self.ftype()? != FileType::File && self.ftype()? != FileType::Symlink {
            return Err(FsError::NotFile);
        }

        let mut writer = INodeData::new(self.self_ref(), offset);

        Ok(writer.write(buf, true)?)
    }

    fn poll(
        &self,
        _poll_table: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        // Regular files are always ready on POLL
        let mut ret = PollEventFlags::empty();
        if flags.contains(PollEventFlags::READ) {
            ret.insert(PollEventFlags::READ);
        }
        if flags.contains(PollEventFlags::WRITE) {
            ret.insert(PollEventFlags::WRITE);
        }

        Ok(ret)
    }

    fn fs(&self) -> Option<Weak<dyn Filesystem>> {
        Some(self.fs())
    }

    fn create(&self, parent: DirEntryItem, name: &str, ftype: FileType) -> Result<DirEntryItem> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        if DirEntIter::new(self.self_ref())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(ftype)?;

        let mut iter = DirEntIter::new_no_skip(self.self_ref());

        if let Err(e) = iter.add_dir_entry(new_inode.as_ext2_inode(), name) {
            logln!("Create failed");
            self.ext2_fs().free_inode(&new_inode.as_ext2_inode());

            Err(e)
        } else {
            Ok(DirEntry::new(parent, new_inode, String::from(name)))
        }
    }

    fn open(&self, _flags: OpenFlags) -> Result<()> {
        logln!("open inode: {:?}", self.read_debug(29).d_inode);

        Ok(())
    }

    fn close(&self, _flags: OpenFlags) {
        logln!("close inode: {:?}", self.read_debug(30).d_inode);
    }

    fn mknode(
        &self,
        parent: DirEntryItem,
        name: &str,
        mode: Mode,
        devid: DevId,
    ) -> Result<INodeItem> {
        let inode = self.create(parent, name, mode.into())?.inode();

        if mode.intersects(Mode::IFBLK | Mode::IFCHR) {
            inode.as_ext2_inode().d_inode_writer().set_rdevid(devid);
        }

        Ok(inode)
    }

    fn symlink(&self, name: &str, target: &str) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        let _me = self.self_ref();

        if DirEntIter::new(self.self_ref())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let new_inode = self.mk_inode(FileType::Symlink)?;
        if let Err(e) = new_inode.write_at(0, target.as_bytes(), OpenFlags::empty()) {
            self.ext2_fs().free_inode(&new_inode.as_ext2_inode());
            return Err(e);
        }

        let mut iter = DirEntIter::new_no_skip(self.self_ref());

        if let Err(e) = iter.add_dir_entry(&new_inode.as_ext2_inode(), name) {
            self.ext2_fs().free_inode(&new_inode.as_ext2_inode());
            Err(e)
        } else {
            Ok(())
        }
    }

    fn link(&self, name: &str, target: INodeItem) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        if target.ftype()? == FileType::Dir {
            return Err(FsError::IsDir);
        }

        if DirEntIter::new(self.self_ref())
            .find(|e| e.name() == name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        let mut iter = DirEntIter::new_no_skip(self.self_ref());

        iter.add_dir_entry(
            &self.ext2_fs().get_inode(target.id()?).as_ext2_inode(),
            name,
        )?;

        Ok(())
    }

    fn rename(&self, old: DirEntryItem, new_name: &str) -> Result<()> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        if self.node.read().d_inode().hl_count() == 0 {
            return Err(FsError::NotSupported);
        }

        if old
            .inode()
            .as_ext2_inode()
            .read_debug(31)
            .d_inode()
            .hl_count()
            == 0
        {
            return Err(FsError::NotSupported);
        }

        if DirEntIter::new(self.self_ref())
            .find(|e| e.name() == new_name)
            .is_some()
        {
            return Err(FsError::EntryExists);
        }

        if let Some(old_parent) = old.parent() {
            if old_parent.inode().ftype()? != FileType::Dir {
                return Err(FsError::NotDir);
            }

            let mut iter = DirEntIter::new_no_skip(self.self_ref());

            iter.add_dir_entry(old.inode().as_ext2_inode(), new_name)?;

            iter = DirEntIter::new_no_skip(old_parent.inode().as_ext2_inode_arc());

            iter.remove_dir_entry(old.name().as_str())?;
        } else {
            return Err(FsError::NotSupported);
        }

        Ok(())
    }

    fn chmod(&self, mode: Mode) -> Result<()> {
        let mut node = self.d_inode_writer();

        node.set_perm(mode.bits() as u16);

        logln5!("do chmod! == {:#o}", node.perm());

        Ok(())
    }

    fn utime(&self, times: &[Timespec; 2]) -> Result<()> {
        logln5!(
            "times: {:?} {} {}",
            times,
            times[0].is_now(),
            times[1].is_now()
        );
        let mut node = self.d_inode_writer();

        if !times[0].is_omit() {
            let access = if !times[0].is_now() {
                times[0].secs as u32
            } else {
                unix_timestamp() as u32
            };

            logln5!("utime: last access: {}", access);
            node.set_last_access(access);
        }

        if !times[1].is_omit() {
            let modif = if !times[1].is_now() {
                times[1].secs as u32
            } else {
                unix_timestamp() as u32
            };

            logln5!("utime: last modif: {}", modif);
            node.set_last_modification(modif);
        }

        Ok(())
    }

    fn truncate(&self, size: usize) -> Result<()> {
        if self.ftype()? != FileType::File {
            return Err(FsError::NotFile);
        }

        let current_size = self.node.read().d_inode().size_lower() as usize;

        if size > current_size {
            let mut node = self.d_inode_writer();

            node.set_size_lower(size as u32);

            logln_disabled!(
                "truncate: fd: {} size: {} {:?}",
                node.id(),
                size,
                node.locked.d_inode
            );

            Ok(())
        } else {
            self.node.write().free_blocks_from(&self.ext2_fs(), size);

            if size == 0 {
                assert_eq!(self.node.read().d_inode().sector_count(), 0);
            }

            Ok(())
        }
    }

    fn dir_ent(&self, parent: DirEntryItem, idx: usize) -> Result<Option<DirEntryItem>> {
        if self.ftype()? != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let mut iter = DirEntIter::new(self.self_ref());

        if let Some(de) = iter.nth(idx) {
            Ok(Some(self.mk_dirent(parent, de)))
        } else {
            Ok(None)
        }
    }

    fn dir_iter(
        &self,
        parent: DirEntryItem,
    ) -> Option<Arc<dyn crate::kernel::fs::vfs::DirEntIter>> {
        if self.ftype().ok()? != FileType::Dir {
            return None;
        }

        Some(Arc::new(SysDirEntIter::new(parent, self.self_ref())))
    }

    fn device_id(&self) -> Option<DevId> {
        if ![FileType::Block, FileType::Char].contains(&self.ftype().ok()?) {
            return None;
        }

        let id = self.node.read().d_inode().get_rdevid();

        if id != 0 {
            Some(id)
        } else {
            None
        }
    }

    fn sync(&self) -> Result<()> {
        let mut pages = self.dirty_list.lock();

        for page in pages.iter() {
            logln!("syncing page to storage");
            page.flush_to_storage(&page);
            logln!("synced page to storage");
        }

        pages.clear();

        self.read().sync_blocks(&self.ext2_fs());

        Ok(())
    }

    fn as_cacheable(&self) -> Option<Arc<dyn CachedAccess>> {
        if self.ftype().unwrap() == FileType::File {
            Some(self.self_ref())
        } else {
            None
        }
    }

    fn as_mappable(&self) -> Option<Arc<dyn MappedAccess>> {
        if self.ftype().unwrap() == FileType::File {
            Some(self.self_ref())
        } else {
            None
        }
    }

    fn debug(&self) {
        logln_disabled!("INode debug: {:?}", self.read_debug(32).d_inode());
    }
}
