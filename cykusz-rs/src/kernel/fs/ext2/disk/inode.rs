#![allow(dead_code)]

#[derive(Copy, Clone, PartialEq)]
pub enum FileType {
    Fifo,
    CharDev,
    Dir,
    BlockDev,
    File,
    Symlink,
    Socket,
    Unknown,
}

impl From<u16> for FileType {
    fn from(v: u16) -> Self {
        let t = v >> 12;

        match t {
            0x1 => FileType::Fifo,
            0x2 => FileType::CharDev,
            0x4 => FileType::Dir,
            0x6 => FileType::BlockDev,
            0x8 => FileType::File,
            0xA => FileType::Symlink,
            0xC => FileType::Socket,
            _ => FileType::Unknown,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct INode {
    type_and_perm: u16,
    user_id: u16,
    size_lower: u32,
    last_access: u32,
    creation_time: u32,
    last_modification: u32,
    deletion_time: u32,
    group_id: u16,
    hl_count: u16,
    sector_count: u32,
    flags: u32,
    os_specific: u32,
    data_ptr: [u32; 15],
    gen_number: u32,
    ext_attr_block: u32,
    size_or_acl: u32,
    fragment_address: u32,
    os_specific2: [u8; 12],
}

impl INode {
    pub fn ftype(&self) -> FileType {
        self.type_and_perm.into()
    }
    pub fn type_and_perm(&self) -> u16 {
        self.type_and_perm
    }
    pub fn user_id(&self) -> u16 {
        self.user_id
    }
    pub fn size_lower(&self) -> u32 {
        self.size_lower
    }
    pub fn last_access(&self) -> u32 {
        self.last_access
    }
    pub fn creation_time(&self) -> u32 {
        self.creation_time
    }
    pub fn last_modification(&self) -> u32 {
        self.last_modification
    }
    pub fn deletion_time(&self) -> u32 {
        self.deletion_time
    }
    pub fn group_id(&self) -> u16 {
        self.group_id
    }
    pub fn hl_count(&self) -> u16 {
        self.hl_count
    }
    pub fn sector_count(&self) -> u32 {
        self.sector_count
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn os_specific(&self) -> u32 {
        self.os_specific
    }
    pub fn direct_ptrs(&self) -> &[u32] {
        unsafe { &self.data_ptr[..12] }
    }
    pub fn block_ptrs(&self) -> &[u32] {
        unsafe { &self.data_ptr }
    }
    pub fn direct_ptr0(&self) -> u32 {
        self.data_ptr[0]
    }
    pub fn direct_ptr1(&self) -> u32 {
        self.data_ptr[1]
    }
    pub fn direct_ptr2(&self) -> u32 {
        self.data_ptr[2]
    }
    pub fn direct_ptr3(&self) -> u32 {
        self.data_ptr[3]
    }
    pub fn direct_ptr4(&self) -> u32 {
        self.data_ptr[4]
    }
    pub fn direct_ptr5(&self) -> u32 {
        self.data_ptr[5]
    }
    pub fn direct_ptr6(&self) -> u32 {
        self.data_ptr[6]
    }
    pub fn direct_ptr7(&self) -> u32 {
        self.data_ptr[7]
    }
    pub fn direct_ptr8(&self) -> u32 {
        self.data_ptr[8]
    }
    pub fn direct_ptr9(&self) -> u32 {
        self.data_ptr[9]
    }
    pub fn direct_ptr10(&self) -> u32 {
        self.data_ptr[10]
    }
    pub fn direct_ptr11(&self) -> u32 {
        self.data_ptr[11]
    }
    pub fn s_indir_ptr(&self) -> u32 {
        self.data_ptr[12]
    }
    pub fn d_indir_ptr(&self) -> u32 {
        self.data_ptr[13]
    }
    pub fn t_indir_ptr(&self) -> u32 {
        self.data_ptr[14]
    }
    pub fn gen_number(&self) -> u32 {
        self.gen_number
    }
    pub fn ext_attr_block(&self) -> u32 {
        self.ext_attr_block
    }
    pub fn size_or_acl(&self) -> u32 {
        self.size_or_acl
    }
    pub fn fragment_address(&self) -> u32 {
        self.fragment_address
    }
    pub fn os_specific2(&self) -> &[u8] {
        &self.os_specific2
    }
}
