use crate::{OpenFlags, SyscallError};

#[derive(Debug, Copy, Clone)]
pub struct SockTypeFlags(u64);

impl SockTypeFlags {
    pub fn new(v: u64) -> SockTypeFlags {
        SockTypeFlags(v)
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct SockFlags: u64 {
        const NONBLOCK = 0o4000;
        const CLOEXEC = 0o2000000;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct MsgFlags: u64 {
        const MSG_OOB = 0x1;
        const MSG_PEEK = 0x2;
        const MSG_DONTROUTE = 0x4;
        const MSG_CTRUNC = 0x8;
        const MSG_TRUNC = 0x20;
        const MSG_EOR = 0x80;
        const MSG_WAITALL = 0x100;
        const MSG_FIN = 0x200;
        const MSG_CONFIRM = 0x800;
        const MSG_NOSIGNAL = 0x4000;

        // Linux extensions.
        const MSG_DONTWAIT = 0x40;
        const MSG_MORE = 0x8000;
        const MSG_FASTOPEN = 0x20000000;
        const MSG_CMSG_CLOEXEC = 0x40000000;
    }
}

impl From<OpenFlags> for MsgFlags {
    fn from(value: OpenFlags) -> Self {
        if value.contains(OpenFlags::NONBLOCK) {
            MsgFlags::MSG_DONTWAIT
        } else {
            MsgFlags::empty()
        }
    }
}

#[repr(u64)]
#[derive(Debug, Copy, Clone)]
pub enum SockType {
    Unknown = 0,
    Stream = 1,
    Dgram = 2,
    Raw = 3,
    SeqPacket = 5,
}

#[repr(u16)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum SockDomain {
    AfUnspec = 0,
    AfUnix = 1,
    AfInet = 2,
    AfInet6 = 10,
    AfNetLink = 16,
    AfBridge = 7,
    AfPacket = 17,
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
        match value.0 & !(SockFlags::all().bits()) {
            1 => SockType::Stream,
            2 => SockType::Dgram,
            3 => SockType::Raw,
            5 => SockType::SeqPacket,
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
            2 => Ok(SockDomain::AfInet),
            10 => Ok(SockDomain::AfInet6),
            1 => Ok(SockDomain::AfUnix),
            0 => Ok(SockDomain::AfUnspec),
            16 => Ok(SockDomain::AfNetLink),
            7 => Ok(SockDomain::AfBridge),
            17 => Ok(SockDomain::AfPacket),
            _ => Err(SyscallError::EINVAL),
        }
    }
}
pub struct SockAddrPtr(*mut ());

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SockAddr {
    pub sa_family: SockDomain,
    pub sa_data: [u8; 14],
}

impl SockAddrPtr {
    pub fn new(addr: *mut ()) -> SockAddrPtr {
        SockAddrPtr(addr)
    }

    pub fn as_sock_addr_in(&self) -> &SockAddrIn {
        unsafe { &*(self.0 as *const SockAddrIn) }
    }

    pub fn as_sock_addr_in_mut(&mut self) -> &mut SockAddrIn {
        unsafe { &mut *(self.0 as *mut SockAddrIn) }
    }

    pub fn as_sock_addr_un(&self) -> &SockAddrUn {
        unsafe { &*(self.0 as *const SockAddrUn) }
    }

    pub fn as_sock_addr_un_mut(&mut self) -> &mut SockAddrUn {
        unsafe { &mut *(self.0 as *mut SockAddrUn) }
    }

    pub fn is_null(&self) -> bool {
        self.0 == core::ptr::null_mut()
    }

    pub fn addr(&self) -> usize {
        self.0 as usize
    }
}

impl SockAddr {
    pub fn as_sock_addr_in(&self) -> &SockAddrIn {
        unsafe { &*(self as *const _ as *const SockAddrIn) }
    }

    pub fn as_sock_addr_in_mut(&mut self) -> &mut SockAddrIn {
        unsafe { &mut *(self as *mut _ as *mut SockAddrIn) }
    }
}

#[repr(C)]
#[derive(Default, Debug)]
pub struct InAddr {
    pub s_addr: NetU32,
}

#[repr(C)]
#[derive(Default, Debug)]
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
}

impl SockAddrIn {
    pub fn port(&self) -> u16 {
        self.sin_port.value()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SockAddrUn {
    pub sin_family: SockDomain,
    pub sun_path: [u8; 108],
}

impl SockAddrUn {
    pub fn new(path: &str) -> Result<SockAddrUn, SyscallError> {
        let mut addr = SockAddrUn {
            sin_family: SockDomain::AfUnix,
            sun_path: [0u8; 108],
        };
        let path = path.as_bytes();
        if path.len() >= 108 {
            return Err(SyscallError::EACCES);
        }
        addr.sun_path[..path.len()].copy_from_slice(path);

        Ok(addr)
    }

    pub fn path(&self) -> &str {
        self.sun_path
            .iter()
            .enumerate()
            .find(|e| *e.1 == 0)
            .and_then(|(idx, _)| Some(core::str::from_utf8(&self.sun_path[..idx])))
            .or_else(|| Some(core::str::from_utf8(&self.sun_path)))
            .unwrap()
            .expect("Invalid unix path")
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
    pub msg_name: SockAddrPtr,
    pub msg_namelen: u32,
    pub msg_iov: *const IoVec,
    pub msg_iovlen: usize,
    pub msg_control: *const (),
    pub msg_controllen: usize,
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

    pub fn sock_addr_in(&self) -> Option<&SockAddrIn> {
        if !self.msg_name.is_null()
            && self.msg_namelen as usize == core::mem::size_of::<SockAddrIn>()
        {
            Some(self.msg_name.as_sock_addr_in())
        } else {
            None
        }
    }

    pub fn sock_addr_in_mut(&mut self) -> Option<&mut SockAddrIn> {
        if !self.msg_name.is_null()
            && self.msg_namelen as usize == core::mem::size_of::<SockAddrIn>()
        {
            Some(self.msg_name.as_sock_addr_in_mut())
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
    pub cmsg_len: usize,
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
