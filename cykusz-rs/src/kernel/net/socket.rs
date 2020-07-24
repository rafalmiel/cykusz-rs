use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use crate::kernel::net::ip::Ip4;

pub fn tcp_bind(port: u32) -> Option<Arc<dyn INode>> {
    crate::kernel::net::tcp::socket::bind(port)
}

pub fn udp_bind(port: u32) -> Option<Arc<dyn INode>> {
    crate::kernel::net::udp::socket::bind(port)
}

pub fn tcp_connect(host: Ip4, port: u32) -> Option<Arc<dyn INode>> {
    crate::kernel::net::tcp::socket::connect(host, port)
}

pub fn udp_connect(host: Ip4, port: u32) -> Option<Arc<dyn INode>> {
    crate::kernel::net::udp::socket::connect(host, port)
}
