use alloc::sync::Arc;

use crate::kernel::net::ip::Ip4;
use crate::kernel::net::udp::socket::Socket;

pub fn udp_bind(port: u32) -> Option<Arc<Socket>> {
    crate::kernel::net::udp::socket::bind(port)
}

pub fn udp_connect(host: Ip4, port: u32) -> Option<Arc<Socket>> {
    crate::kernel::net::udp::socket::connect(host, port)
}
