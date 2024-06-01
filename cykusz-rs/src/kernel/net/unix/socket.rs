use crate::kernel::fs::inode::INode;
use crate::kernel::net::socket::SocketService;
use alloc::sync::{Arc, Weak};

pub struct Socket {
    self_ref: Weak<Socket>,
}

impl Socket {
    pub fn new_unbound() -> Arc<Socket> {
        Arc::new_cyclic(|me| Socket {
            self_ref: me.clone(),
        })
    }
}

impl SocketService for Socket {}

impl INode for Socket {}
