#![allow(dead_code)]

use crate::kernel::mm::VirtAddr;

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum FsState {
    Clean = 1,
    Dirty = 2,
}

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum ErrorAction {
    Ignore = 1,
    RemountRO = 2,
    Panic = 3,
}

bitflags! {
    pub struct OptFeatures: u32 {
        const PREALLOCATE_DIR_BLOCKS = 0x0001;
        const AFS_INODES = 0x0002;
        const HAS_JOURNAL = 0x0004;
        const INODE_EXT_ATTR = 0x0008;
        const FS_RESIZE = 0x0010;
        const DIR_HASH_IDX = 0x0020;
    }
}

bitflags! {
    pub struct ReqFeatures: u32 {
        const COMPRESSION_USED = 0x0001;
        const DIRENT_TYPE_FIELD = 0x0002;
        const FS_REPLY_JOURNAL = 0x0004;
        const FS_USES_JOURNAL = 0x0008;
    }
}

bitflags! {
    pub struct RoFeatures: u32 {
        const SPARSE_SUPERBLOCKS = 0x0001;
        const FS_64BIT_FILESIZE = 0x0002;
        const DIR_BINARY_TREE = 0x0004;
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Superblock {
    inodes: u32,
    blocks: u32,
    su_blocks: u32,
    free_blocks: u32,
    free_inodes: u32,
    superblock_block: u32,
    block_size: u32,
    fragment_size: u32,
    blocks_in_group: u32,
    fragments_in_group: u32,
    inodes_in_group: u32,
    last_mount_time: u32,
    last_written_time: u32,
    mounts_since_check: u16,
    mounts_allowed: u16,
    ext_sig: u16,
    fs_state: FsState,
    error_action: ErrorAction,
    minor_ver: u16,
    last_check: u32,
    check_interval: u32,
    os_id: u32,
    major_ver: u32,
    rsv_user_id: u16,
    rsv_group_id: u16,

    //Extended Superblock fields
    first_nonresv_inode: u32,
    inode_size: u16,
    this_block_group: u16,
    opt_features: OptFeatures,
    req_features: ReqFeatures,
    ro_features: RoFeatures,
    fs_id: [u8; 16],
    vol_name: [u8; 16],
    last_path: [u8; 64],
    compression_algo: [u8; 4],
    file_blocks_preallocate: u8,
    dir_blocks_preallocate: u8,
    _unused: u16,
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_dev: u32,
    orphan_list_head: u32,
    _unused2: [u8; 1024 - 236],
}

impl Default for Superblock {
    fn default() -> Superblock {
        Superblock {
            inodes: 0,
            blocks: 0,
            su_blocks: 0,
            free_blocks: 0,
            free_inodes: 0,
            superblock_block: 0,
            block_size: 0,
            fragment_size: 0,
            blocks_in_group: 0,
            fragments_in_group: 0,
            inodes_in_group: 0,
            last_mount_time: 0,
            last_written_time: 0,
            mounts_since_check: 0,
            mounts_allowed: 0,
            ext_sig: 0,
            fs_state: FsState::Clean,
            error_action: ErrorAction::Ignore,
            minor_ver: 0,
            last_check: 0,
            check_interval: 0,
            os_id: 0,
            major_ver: 0,
            rsv_user_id: 0,
            rsv_group_id: 0,
            first_nonresv_inode: 0,
            inode_size: 0,
            this_block_group: 0,
            opt_features: OptFeatures::empty(),
            req_features: ReqFeatures::empty(),
            ro_features: RoFeatures::empty(),
            fs_id: [0u8; 16],
            vol_name: [0u8; 16],
            last_path: [0u8; 64],
            compression_algo: [0u8; 4],
            file_blocks_preallocate: 0,
            dir_blocks_preallocate: 0,
            _unused: 0,
            journal_id: [0u8; 16],
            journal_inode: 0,
            journal_dev: 0,
            orphan_list_head: 0,
            _unused2: [0u8; 1024 - 236],
        }
    }
}

impl Superblock {
    pub fn self_addr(&self) -> VirtAddr {
        VirtAddr(self as *const _ as usize)
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.self_addr().as_bytes(core::mem::size_of::<Self>()) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { self.self_addr().as_bytes_mut(core::mem::size_of::<Self>()) }
    }

    pub fn group_count(&self) -> usize {
        let ipg = self.inodes_in_group() as usize;

        (self.inodes() as usize + ipg - 1) / ipg
    }

    pub fn inodes(&self) -> u32 {
        self.inodes
    }
    pub fn blocks(&self) -> u32 {
        self.blocks
    }
    pub fn su_blocks(&self) -> u32 {
        self.su_blocks
    }
    pub fn free_blocks(&self) -> u32 {
        self.free_blocks
    }
    pub fn dec_free_blocks(&mut self) {
        self.free_blocks -= 1;
    }
    pub fn inc_free_blocks(&mut self) {
        self.free_blocks += 1;
    }
    pub fn free_inodes(&self) -> u32 {
        self.free_inodes
    }
    pub fn dec_free_inodes(&mut self) {
        self.free_inodes -= 1;
    }
    pub fn inc_free_inodes(&mut self) {
        self.free_inodes += 1;
    }
    pub fn superblock_block(&self) -> u32 {
        self.superblock_block
    }
    pub fn block_size(&self) -> usize {
        1024usize << self.block_size
    }
    pub fn fragment_size(&self) -> usize {
        1024usize << self.fragment_size
    }
    pub fn blocks_in_group(&self) -> u32 {
        self.blocks_in_group
    }
    pub fn fragments_in_group(&self) -> u32 {
        self.fragments_in_group
    }
    pub fn inodes_in_group(&self) -> u32 {
        self.inodes_in_group
    }
    pub fn last_mount_time(&self) -> u32 {
        self.last_mount_time
    }
    pub fn last_written_time(&self) -> u32 {
        self.last_written_time
    }
    pub fn mounts_since_check(&self) -> u16 {
        self.mounts_since_check
    }
    pub fn mounts_allowed(&self) -> u16 {
        self.mounts_allowed
    }
    pub fn ext_sig(&self) -> u16 {
        self.ext_sig
    }
    pub fn fs_state(&self) -> FsState {
        self.fs_state
    }
    pub fn error_action(&self) -> ErrorAction {
        self.error_action
    }
    pub fn minor_ver(&self) -> u16 {
        self.minor_ver
    }
    pub fn last_check(&self) -> u32 {
        self.last_check
    }
    pub fn check_interval(&self) -> u32 {
        self.check_interval
    }
    pub fn os_id(&self) -> u32 {
        self.os_id
    }
    pub fn major_ver(&self) -> u32 {
        self.major_ver
    }
    pub fn rsv_user_id(&self) -> u16 {
        self.rsv_user_id
    }
    pub fn rsv_group_id(&self) -> u16 {
        self.rsv_group_id
    }
    pub fn inode_size(&self) -> u16 {
        self.inode_size
    }
    pub fn inodes_per_block(&self) -> usize {
        self.block_size() / self.inode_size() as usize
    }
    pub fn sectors_per_block(&self) -> usize {
        self.block_size() / 512
    }
    pub fn fs_id(&self) -> &[u8] {
        &self.fs_id
    }
}
