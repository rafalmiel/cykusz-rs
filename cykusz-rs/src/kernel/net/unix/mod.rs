use crate::kernel::sync::{LockApi, Mutex, MutexGuard};
use crate::kernel::utils::node_map::NodeMap;
use spin::Once;

pub mod socket;

pub static SOCKETS: Once<Mutex<NodeMap<socket::Socket>>> = Once::new();

pub fn init() {
    SOCKETS.call_once(|| Mutex::new(NodeMap::new()));
}

pub fn sockets<'a>() -> MutexGuard<'a, NodeMap<socket::Socket>> {
    unsafe { SOCKETS.get_unchecked().lock() }
}
