use crate::SyscallError;

#[derive(Debug, Copy, Clone)]
pub struct SockTypeFlags(u64);

impl SockTypeFlags {
    pub fn new(v: u64) -> SockTypeFlags {
        SockTypeFlags(v)
    }
}

bitflags! {
    pub struct SockFlags: u64 {
        const NONBLOCK = 0x10000;
        const CLOEXEC = 0x20000;
    }
}

bitflags! {
    pub struct MsgFlags: u64 {
        const MSG_CTRUNC = 0x1;
        const MSG_DONTROUTE = 0x2;
        const MSG_EOR = 0x4;
        const MSG_OOB = 0x8;
        const MSG_NOSIGNAL = 0x10;
        const MSG_PEEK = 0x20;
        const MSG_TRUNC = 0x40;
        const MSG_WAITALL = 0x80;
        const MSG_FIN = 0x200;
        const MSG_CONFIRM = 0x800;

        // Linux extensions.
        const MSG_DONTWAIT = 0x1000;
        const MSG_CMSG_CLOEXEC = 0x2000;
        const MSG_MORE = 0x4000;
        const MSG_FASTOPEN = 0x20000000;
    }
}

#[repr(u64)]
#[derive(Debug, Copy, Clone)]
pub enum SockType {
    Unknown = 0,
    Dgram = 1,
    Raw = 2,
    SeqPacket = 3,
    Stream = 4,
    Rdm = 0x40000,
}

#[repr(u32)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum SockDomain {
    AfInet = 1,
    AfInet6 = 2,
    AfUnix = 3,
    AfUnspec = 4,
    AfNetLink = 5,
    AfBridge = 6,
    AfPacket = 13,
}

impl Default for SockDomain {
    fn default() -> Self {
        SockDomain::AfInet
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum SockOption {
    KeepAlive = 6,
}

impl TryFrom<u64> for SockOption {
    type Error = SyscallError;

    fn try_from(value: u64) -> Result<Self, SyscallError> {
        match value {
            6 => Ok(SockOption::KeepAlive),
            _ => Err(SyscallError::EINVAL),
        }
    }
}

impl From<SockTypeFlags> for SockType {
    fn from(value: SockTypeFlags) -> Self {
        match value.0 & !(SockFlags::all().bits) {
            1 => SockType::Dgram,
            2 => SockType::Raw,
            3 => SockType::SeqPacket,
            4 => SockType::Stream,
            0x40000 => SockType::Rdm,
            _ => SockType::Unknown,
        }
    }
}

impl From<SockTypeFlags> for SockFlags {
    fn from(value: SockTypeFlags) -> Self {
        SockFlags::from_bits_truncate(value.0)
    }
}

impl From<SockTypeFlags> for usize {
    fn from(value: SockTypeFlags) -> Self {
        value.0 as usize
    }
}

impl TryFrom<u64> for SockDomain {
    type Error = SyscallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SockDomain::AfInet),
            2 => Ok(SockDomain::AfInet6),
            3 => Ok(SockDomain::AfUnix),
            4 => Ok(SockDomain::AfUnspec),
            5 => Ok(SockDomain::AfNetLink),
            6 => Ok(SockDomain::AfBridge),
            13 => Ok(SockDomain::AfPacket),
            _ => Err(SyscallError::EINVAL),
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct SockAddr {
    pub sa_family: SockDomain,
    pub sa_data: [u8; 14],
}

impl SockAddr {
    pub fn as_sock_addr_in(&self) -> &SockAddrIn {
        unsafe { core::mem::transmute::<&SockAddr, &SockAddrIn>(self) }
    }

    pub fn as_sock_addr_in_mut(&mut self) -> &mut SockAddrIn {
        unsafe { core::mem::transmute::<&mut SockAddr, &mut SockAddrIn>(self) }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct InAddr {
    pub s_addr: NetU32,
}

#[repr(C)]
#[derive(Default)]
pub struct SockAddrIn {
    pub sin_family: SockDomain,
    pub sin_port: NetU16,
    pub sin_addr: InAddr,
    pad: [u8; 8],
}

impl SockAddrIn {
    pub fn new(port: u16, addr: NetU32) -> SockAddrIn {
        SockAddrIn {
            sin_family: SockDomain::AfInet,
            sin_port: NetU16::new(port),
            sin_addr: InAddr { s_addr: addr },
            ..Default::default()
        }
    }

    pub fn from_array(port: u16, addr: &[u8]) -> SockAddrIn {
        let chunks = addr.as_chunks::<4>();
        assert_eq!(chunks.0.len(), 1);
        SockAddrIn {
            sin_family: SockDomain::AfInet,
            sin_port: NetU16::new(port),
            sin_addr: InAddr {
                s_addr: unsafe { core::mem::transmute::<[u8; 4], NetU32>(chunks.0[0]) },
            },
            ..Default::default()
        }
    }

    pub fn into_sock_addr(self) -> SockAddr {
        unsafe { core::mem::transmute::<SockAddrIn, SockAddr>(self) }
    }
}

impl SockAddrIn {
    pub fn port(&self) -> u16 {
        self.sin_port.value()
    }
}

#[repr(C)]
pub struct IpMreq {
    pub imr_multiaddr: InAddr,
    pub imr_interface: InAddr,
    pub imr_sourceaddr: InAddr,
}

#[repr(C)]
pub struct IpMreqN {
    pub imr_multiaddr: InAddr,
    pub imr_interface: InAddr,
    pub imr_ifindex: i32,
}

#[repr(C)]
pub struct InPktInfo {
    pub ipi_ifindex: u32,
    pub ipi_spec_dst: InAddr,
    pub ipi_addr: InAddr,
}

#[repr(C)]
pub struct IpMreqSource {
    pub imr_multiaddr: InAddr,
    pub imr_interface: InAddr,
}

#[repr(C)]
pub struct GroupSourceReq {
    pub gsr_interface: u32,
    pub gsr_group: SockAddrStorage,
    pub gsr_source: SockAddrStorage,
}

#[repr(C)]
pub struct Linger {
    pub l_onoff: i32,
    pub l_linger: i32,
}

#[repr(C)]
pub struct Ucred {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

#[repr(C)]
pub struct IoVec {
    pub iov_base: *const (),
    pub iov_len: usize,
}

impl IoVec {
    pub fn get_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.iov_base as *const u8, self.iov_len) }
    }

    pub fn get_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.iov_base as *mut u8, self.iov_len) }
    }
}

#[repr(C)]
pub struct MsgHdr {
    pub msg_name: *const (),
    pub msg_namelen: u32,
    pub msg_iov: *const IoVec,
    pub msg_iovlen: i32,
    pub msg_control: *const (),
    pub msg_controllen: u32,
    pub msg_flags: i32,
}

impl MsgHdr {
    pub fn iovecs_mut(&mut self) -> &mut [IoVec] {
        unsafe {
            core::slice::from_raw_parts_mut(self.msg_iov as *mut IoVec, self.msg_iovlen as usize)
        }
    }

    pub fn iovecs(&self) -> &[IoVec] {
        unsafe { core::slice::from_raw_parts(self.msg_iov, self.msg_iovlen as usize) }
    }

    pub fn sock_addr(&self) -> Option<&SockAddr> {
        if self.msg_name != core::ptr::null()
            && self.msg_namelen as usize == core::mem::size_of::<SockAddr>()
        {
            unsafe { (self.msg_name as *const SockAddr).as_ref() }
        } else {
            None
        }
    }

    pub fn sock_addr_mut(&mut self) -> Option<&mut SockAddr> {
        if self.msg_name != core::ptr::null()
            && self.msg_namelen as usize == core::mem::size_of::<SockAddr>()
        {
            unsafe { (self.msg_name as *mut SockAddr).as_mut() }
        } else {
            None
        }
    }
}

#[repr(C, packed)]
pub struct SockAddrStorage {
    pub ss_family: NetU32,
    pub __padding: [u8; 128 - core::mem::size_of::<u32>()],
}

#[repr(C, packed)]
pub struct MMsgHdr {
    pub msg_hdr: MsgHdr,
    pub msg_len: u32,
}

#[repr(C, packed)]
pub struct CMsgHdr {
    pub cmsg_len: u32,
    pub cmsg_level: i32,
    pub cmsg_type: i32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct NetU32(u32);

#[derive(Debug, Default, Copy, Clone)]
pub struct NetU16(u16);

#[derive(Debug, Default, Copy, Clone)]
pub struct NetU8(u8);

macro_rules! impl_net (
    ($type_: ident, $src: ident) => {
        impl $type_ {
            pub const fn new(v: $src) -> $type_ {
                if cfg!(target_endian = "little") {
                    $type_(v.swap_bytes())
                } else {
                    $type_(v)
                }
            }

            pub const fn value(self) -> $src {
                if cfg!(target_endian = "little") {
                    self.0.swap_bytes()
                } else {
                    self.0
                }
            }

            pub const fn net_value(self) -> $src {
                self.0
            }
        }
    }
);

impl_net!(NetU32, u32);
impl_net!(NetU16, u16);
impl_net!(NetU8, u8);
