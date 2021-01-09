#![allow(dead_code)]

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
    direct_ptr: [u32; 12],
    s_indir_ptr: u32,
    d_indir_ptr: u32,
    t_indir_ptr: u32,
    gen_number: u32,
    ext_attr_block: u32,
    size_or_acl: u32,
    fragment_address: u32,
    os_specific2: [u8; 12],
}

impl INode {
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
        unsafe { &self.direct_ptr }
    }
    pub fn direct_ptr0(&self) -> u32 {
        self.direct_ptr[0]
    }
    pub fn direct_ptr1(&self) -> u32 {
        self.direct_ptr[1]
    }
    pub fn direct_ptr2(&self) -> u32 {
        self.direct_ptr[2]
    }
    pub fn direct_ptr3(&self) -> u32 {
        self.direct_ptr[3]
    }
    pub fn direct_ptr4(&self) -> u32 {
        self.direct_ptr[4]
    }
    pub fn direct_ptr5(&self) -> u32 {
        self.direct_ptr[5]
    }
    pub fn direct_ptr6(&self) -> u32 {
        self.direct_ptr[6]
    }
    pub fn direct_ptr7(&self) -> u32 {
        self.direct_ptr[7]
    }
    pub fn direct_ptr8(&self) -> u32 {
        self.direct_ptr[8]
    }
    pub fn direct_ptr9(&self) -> u32 {
        self.direct_ptr[9]
    }
    pub fn direct_ptr10(&self) -> u32 {
        self.direct_ptr[10]
    }
    pub fn direct_ptr11(&self) -> u32 {
        self.direct_ptr[11]
    }
    pub fn s_indir_ptr(&self) -> u32 {
        self.s_indir_ptr
    }
    pub fn d_indir_ptr(&self) -> u32 {
        self.d_indir_ptr
    }
    pub fn t_indir_ptr(&self) -> u32 {
        self.t_indir_ptr
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
