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

impl FileType {
    fn encode(&self) -> u16 {
        let f = match self {
            FileType::Fifo => 0x1,
            FileType::CharDev => 0x2,
            FileType::Dir => 0x4,
            FileType::BlockDev => 0x6,
            FileType::File => 0x8,
            FileType::Symlink => 0xA,
            FileType::Socket => 0xC,
            FileType::Unknown => 0x0,
        } as u16;

        f << 12
    }
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
    pub fn set_ftype(&mut self, t: FileType) {
        let mask = 0b1111_1111_1111;
        self.type_and_perm = t.encode() | (self.type_and_perm & mask);
    }
    pub fn type_and_perm(&self) -> u16 {
        self.type_and_perm
    }
    pub fn set_perm(&mut self, perm: u16) {
        let mask = 0b1111_1111_1111;
        self.type_and_perm = (self.type_and_perm & !mask) | (perm & mask);
    }
    pub fn user_id(&self) -> u16 {
        self.user_id
    }
    pub fn set_user_id(&mut self, user_id: u16) {
        self.user_id = user_id;
    }
    pub fn size_lower(&self) -> u32 {
        self.size_lower
    }
    pub fn set_size_lower(&mut self, size: u32) {
        self.size_lower = size;
    }
    pub fn last_access(&self) -> u32 {
        self.last_access
    }
    pub fn set_last_access(&mut self, access: u32) {
        self.last_access = access;
    }
    pub fn creation_time(&self) -> u32 {
        self.creation_time
    }
    pub fn set_creation_time(&mut self, creation: u32) {
        self.creation_time = creation;
    }
    pub fn last_modification(&self) -> u32 {
        self.last_modification
    }
    pub fn set_last_modification(&mut self, modif: u32) {
        self.last_modification = modif;
    }
    pub fn deletion_time(&self) -> u32 {
        self.deletion_time
    }
    pub fn set_deletion_time(&mut self, deletion: u32) {
        self.deletion_time = deletion;
    }
    pub fn group_id(&self) -> u16 {
        self.group_id
    }
    pub fn set_group_id(&mut self, group_id: u16) {
        self.group_id = group_id;
    }
    pub fn hl_count(&self) -> u16 {
        self.hl_count
    }
    pub fn inc_hl_count(&mut self) {
        self.hl_count += 1;
    }
    pub fn dec_hl_count(&mut self) {
        self.hl_count -= 1;
    }
    pub fn sector_count(&self) -> u32 {
        self.sector_count
    }
    pub fn set_sector_count(&mut self, count: u32) {
        self.sector_count = count;
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
    pub fn os_specific(&self) -> u32 {
        self.os_specific
    }
    pub fn direct_ptrs(&self) -> &[u32] {
        unsafe { &self.data_ptr[..12] }
    }
    pub fn direct_ptrs_mut(&mut self) -> &mut [u32] {
        unsafe { &mut self.data_ptr[..12] }
    }
    pub fn block_ptrs(&self) -> &[u32] {
        unsafe { &self.data_ptr }
    }
    pub fn block_ptrs_mut(&mut self) -> &mut [u32] {
        unsafe { &mut self.data_ptr }
    }
    pub fn s_indir_ptr(&self) -> u32 {
        self.data_ptr[12]
    }
    pub fn set_s_indir_ptr(&mut self, ptr: u32) {
        self.data_ptr[12] = ptr;
    }
    pub fn d_indir_ptr(&self) -> u32 {
        self.data_ptr[13]
    }
    pub fn set_d_indir_ptr(&mut self, ptr: u32) {
        self.data_ptr[13] = ptr;
    }
    pub fn t_indir_ptr(&self) -> u32 {
        self.data_ptr[14]
    }
    pub fn set_t_indir_ptr(&mut self, ptr: u32) {
        self.data_ptr[14] = ptr;
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
