use alloc::sync::Arc;

use syscall_defs::{SyscallError, SyscallResult};
use syscall_defs::net::{
    MsgFlags, MsgHdr, SockAddr, SockDomain, SockOption, SockType, SockTypeFlags,
};
use syscall_defs::SyscallError::ENOTSUP;

use crate::kernel::fs::inode::INode;
use crate::kernel::net::ip::{Ip, Ip4};
use crate::kernel::net::Packet;

pub fn new(domain: SockDomain, typ: SockTypeFlags) -> Result<Arc<dyn INode>, SyscallError> {
    if domain != SockDomain::AfInet {
        return Err(SyscallError::ENOTSUP);
    }

    logln4!("new socket: {:?} {:?}", domain, SockType::from(typ));

    match SockType::from(typ) {
        SockType::Stream => Ok(crate::kernel::net::tcp::socket::Socket::new_unbound()),
        SockType::Dgram => Ok(crate::kernel::net::udp::socket::Socket::new_unbound()),
        _ => Err(ENOTSUP),
    }
}

pub trait SocketService: Sync + Send {
    fn process_packet(&self, packet: Packet<Ip>);
    fn port_unreachable(&self, port: u32, dst_port: u32);

    fn listen(&self, _backlog: i32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn accept(
        &self,
        _sock_addr: Option<&mut SockAddr>,
        _addrlen: Option<&mut u32>,
    ) -> Result<Arc<dyn SocketService>, SyscallError> {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn bind(&self, _sock_addr: &SockAddr, _addrlen: u32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn connect(&self, _sock_addr: &SockAddr, _addrlen: u32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn msg_send(&self, _hdr: &MsgHdr, _flags: MsgFlags) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn msg_recv(&self, _hdr: &mut MsgHdr, _flags: MsgFlags) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn src_port(&self) -> u32;
    fn target(&self) -> Ip4;
    fn set_src_port(&self, _src_port: u32) {}

    fn set_socket_option(
        &self,
        _layer: i32,
        _option: SockOption,
        _buffer: *const (),
        _size: u32,
    ) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn get_socket_option(
        &self,
        _layer: i32,
        _option: SockOption,
        _buffer: *mut (),
        _socklen: Option<&mut u32>,
    ) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn as_inode(&self) -> Option<Arc<dyn INode>> {
        None
    }
}
