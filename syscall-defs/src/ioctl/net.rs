use crate::net::SockAddr;

pub const SIOCGIFNAME: usize = 0x8910;
pub const SIOCGIFCONF: usize = 0x8912;
pub const SIOCGIFFLAGS: usize = 0x8913;
pub const SIOCSIFFLAGS: usize = 0x8914;
pub const SIOCGIFINDEX: usize = 0x8933;

pub const IF_NAMESIZE: usize = 16;
pub const IFNAMSIZ: usize = IF_NAMESIZE;

#[repr(C)]
pub struct NameIndex {
    pub if_index: u32,
    pub if_name: *mut u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct IfMap {
    pub mem_start: u64,
    pub mem_end: u64,
    pub base_addr: u16,
    pub irq: u8,
    pub dma: u8,
    pub port: u8,
}

bitflags! {
    pub struct IfrFlags: u16 {
        const IFF_UP = 0x1;
        const IFF_BROADCAST = 0x2;
        const IFF_LOOPBACK = 0x8;
        const IFF_POINTPOINT = 0x10;
        const IFF_RUNNING = 0x40;
    }
}
#[repr(C)]
pub union IfReqU {
    pub ifr_addr: SockAddr,
    pub ifr_dstaddr: SockAddr,
    pub ifr_broadaddr: SockAddr,
    pub ifr_netmask: SockAddr,
    pub ifr_hdaddr: SockAddr,
    pub ifr_flags: IfrFlags,
    pub ifr_ifindex: i32,
    pub ifr_metric: i32,
    pub ifr_mtu: i32,
    pub ifr_map: IfMap,
    pub ifr_slave: [u8; IFNAMSIZ],
    pub ifr_newname: [u8; IFNAMSIZ],
    pub ifr_data: *mut u8,
}

#[repr(C)]
pub struct IfReq {
    pub ifr_name: [u8; IFNAMSIZ],
    pub ifrequ: IfReqU,
}

#[repr(C)]
pub union IfConfU {
    pub ifcu_buf: *mut u8,
    pub ifcu_req: *mut IfReq,
}

#[repr(C)]
pub struct IfConf {
    pub ifc_len: i32,
    pub ifc_ifcu: IfConfU,
}

impl IfConf {
    pub fn get_req_array(&mut self) -> Option<&mut [IfReq]> {
        let req_size = core::mem::size_of::<IfReq>();
        if req_size > self.ifc_len as usize {
            None
        } else {
            unsafe {
                Some(core::slice::from_raw_parts_mut(
                    self.ifc_ifcu.ifcu_req,
                    req_size.div_floor(req_size),
                ))
            }
        }
    }
}
