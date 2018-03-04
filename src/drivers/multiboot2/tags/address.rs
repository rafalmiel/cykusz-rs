#[repr(C)]
pub struct Address {
    pub typ:            u16,
    pub flags:          u16,
    pub size:           u32,
    pub header_addr:    u32,
    pub load_addr:      u32,
    pub load_end_addr:  u32,
    pub bss_end_addr:   u32
}
