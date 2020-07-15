use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::net::udp::{Udp, UdpService};
use crate::kernel::net::{Packet, PacketHeader};
use crate::kernel::utils::buffer::BufferQueue;

pub struct Socket {
    buffer: BufferQueue,
    port: u32,
}

impl Socket {
    pub fn new(port: u32) -> Socket {
        Socket {
            buffer: BufferQueue::new(4 * 1024),
            port,
        }
    }
}

impl INode for Socket {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buffer.read_data(buf))
    }

    fn close(&self) {
        crate::kernel::net::udp::release_handler(self.port);
    }
}

impl UdpService for Socket {
    fn process_packet(&self, packet: Packet<Udp>) {
        self.buffer.append_data(packet.header().data());
    }

    fn port_unreachable(&self, _port: u32) {
        unimplemented!()
    }
}

pub fn udp_bind(port: u32) -> Option<Arc<Socket>> {
    let socket = Arc::new(Socket::new(port));

    if crate::kernel::net::udp::register_handler(port, socket.clone()) {
        Some(socket)
    } else {
        None
    }
}
