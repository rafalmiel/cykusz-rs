use alloc::sync::Arc;

use syscall_defs::net::{
    MsgFlags, MsgHdr, SockAddrPtr, SockDomain, SockOption, SockType, SockTypeFlags,
};
use syscall_defs::SyscallError::ENOTSUP;
use syscall_defs::{SyscallError, SyscallResult};

use crate::kernel::fs::inode::INode;
use crate::kernel::net::ip::{Ip, Ip4};
use crate::kernel::net::Packet;

pub fn new(domain: SockDomain, typ: SockTypeFlags) -> Result<Arc<dyn INode>, SyscallError> {
    logln4!("new socket: {:?} {:?}", domain, SockType::from(typ));

    match (domain, SockType::from(typ)) {
        (SockDomain::AfInet, SockType::Stream) => {
            Ok(crate::kernel::net::tcp::socket::Socket::new_unbound())
        }
        (SockDomain::AfInet, SockType::Dgram) => {
            Ok(crate::kernel::net::udp::socket::Socket::new_unbound())
        }
        (SockDomain::AfUnix, _st @ (SockType::Stream | SockType::Dgram)) => {
            Ok(crate::kernel::net::unix::socket::Socket::new_unbound())
        }
        _ => Err(ENOTSUP),
    }
}

pub trait SocketService: Sync + Send {
    fn listen(&self, _backlog: i32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn accept(
        &self,
        _sock_addr: SockAddrPtr,
        _addrlen: Option<&mut u32>,
    ) -> Result<Arc<dyn SocketService>, SyscallError> {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn bind(&self, _sock_addr: SockAddrPtr, _addrlen: u32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn connect(&self, _sock_addr: SockAddrPtr, _addrlen: u32) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn msg_send(&self, _hdr: &MsgHdr, _flags: MsgFlags) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

    fn msg_recv(&self, _hdr: &mut MsgHdr, _flags: MsgFlags) -> SyscallResult {
        Err(SyscallError::EOPNOTSUPP)
    }

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

pub trait NetSocketService: SocketService {
    fn process_packet(&self, packet: Packet<Ip>);
    fn port_unreachable(&self, port: u32, dst_port: u32);
    fn src_port(&self) -> u32;
    fn target(&self) -> Ip4;
    fn set_src_port(&self, _src_port: u32) {}
}
