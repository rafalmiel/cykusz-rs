use crate::kernel::fs::inode::INode;
use crate::kernel::net::socket::SocketService;
use crate::kernel::sync::Mutex;
use alloc::sync::{Arc, Weak};
use syscall_defs::net::{MsgFlags, MsgHdr, SockAddrPtr};
use syscall_defs::{SyscallError, SyscallResult};

struct SocketData {}

pub struct Socket {
    self_ref: Weak<Socket>,
    data: Mutex<SocketData>,
}

impl Socket {
    pub fn new_unbound() -> Arc<Socket> {
        Arc::new_cyclic(|me| Socket {
            self_ref: me.clone(),
            data: Mutex::new(SocketData {}),
        })
    }
}

impl SocketService for Socket {
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

    fn as_inode(&self) -> Option<Arc<dyn INode>> {
        Some(self.self_ref.upgrade()?.clone())
    }
}

impl INode for Socket {}
