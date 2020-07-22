use alloc::sync::Arc;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::net::tcp::{Tcp, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader};

pub struct Socket {
    src_port: AtomicU32,
}

impl Socket {
    pub fn new(port: u32) -> Socket {
        Socket {
            src_port: AtomicU32::new(port),
        }
    }

    pub fn set_src_port(&self, port: u32) {
        self.src_port.store(port, Ordering::SeqCst);
    }

    pub fn src_port(&self) -> u32 {
        self.src_port.load(Ordering::SeqCst)
    }
}

impl INode for Socket {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Ok(0)
    }

    fn poll_listen(&self, _listen: bool) -> Result<bool> {
        Ok(false)
    }

    fn poll_unlisten(&self) -> Result<()> {
        Ok(())
    }

    fn close(&self) {
        crate::kernel::net::tcp::release_handler(self.src_port());
    }
}
impl TcpService for Socket {
    fn process_packet(&self, packet: Packet<Tcp>) {
        let ip = packet.downgrade();

        let ip_hdr = ip.header();
        let hdr = packet.header();

        if hdr.flag_syn() {
            let mut out_packet = crate::kernel::net::tcp::create_packet(
                hdr.dst_port(),
                hdr.src_port(),
                0,
                ip_hdr.src_ip,
            );

            let out_hdr = out_packet.header_mut();

            out_hdr.set_flag_ack(true);
            out_hdr.set_flag_syn(true);
            out_hdr.set_seq_nr(12345);
            out_hdr.set_ack_nr(hdr.seq_nr().wrapping_add(1));
            out_hdr.set_urgent_ptr(0);
            out_hdr.set_window(4096);

            crate::kernel::net::tcp::send_packet(out_packet);
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);
    }
}

pub fn bind(port: u32) -> Option<Arc<dyn INode>> {
    let socket = Arc::new(Socket::new(port));

    if crate::kernel::net::tcp::register_handler(port, socket.clone()) {
        Some(socket)
    } else {
        None
    }
}
