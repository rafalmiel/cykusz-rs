use alloc::sync::Arc;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::tcp::{Tcp, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader};
use crate::kernel::sync::Spin;

#[derive(Default)]
struct SocketData {
    their_seq: u32,
    our_seq: u32,
    src_port: u16,
    dst_port: u16,
    target: Ip4,
}

impl SocketData {
    pub fn new() -> SocketData {
        SocketData::default()
    }

    fn process(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let ip = packet.downgrade();

        let ip_hdr = ip.header();
        let hdr = packet.header();

        self.src_port = hdr.dst_port();
        self.dst_port = hdr.src_port();
        self.target = ip_hdr.src_ip;

        if hdr.flag_syn() {
            println!("[ TCP ] Received SYN");
            let mut out_packet = crate::kernel::net::tcp::create_packet(
                self.src_port,
                self.dst_port,
                0,
                self.target,
            );

            let out_hdr = out_packet.header_mut();

            self.our_seq = 12345;
            self.their_seq = hdr.seq_nr().wrapping_add(1);

            out_hdr.set_flag_ack(true);
            out_hdr.set_flag_syn(true);
            out_hdr.set_seq_nr(self.our_seq);
            out_hdr.set_ack_nr(self.their_seq);
            out_hdr.set_urgent_ptr(0);
            out_hdr.set_window(4096);

            Some(out_packet)
        } else if hdr.flag_fin() {
            println!("[ TCP ] Received FIN");
            let mut out_packet = crate::kernel::net::tcp::create_packet(
                self.src_port,
                self.dst_port,
                0,
                ip_hdr.src_ip,
            );

            let out_hdr = out_packet.header_mut();

            self.our_seq = hdr.ack_nr();
            self.their_seq = hdr.seq_nr().wrapping_add(1);

            out_hdr.set_flag_ack(true);
            out_hdr.set_seq_nr(self.our_seq);
            out_hdr.set_ack_nr(self.their_seq);
            out_hdr.set_window(4096);

            crate::kernel::net::tcp::release_handler(self.src_port as u32);

            Some(out_packet)
        } else if hdr.flag_ack() {
            println!("[ TCP ] Received ACK");
            self.our_seq = hdr.ack_nr();
            self.their_seq = hdr.seq_nr();
            None
        } else {
            None
        }
    }

    fn close(&mut self) {
        println!("[ TCP ] Closing");
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, 0, self.target);

        let mut hdr = out_packet.header_mut();

        hdr.set_flag_fin(true);
        hdr.set_flag_ack(true);
        hdr.set_seq_nr(self.our_seq);
        hdr.set_ack_nr(self.their_seq);
        hdr.set_urgent_ptr(0);
        hdr.set_window(4096);

        crate::kernel::net::tcp::send_packet(out_packet);
    }
}

pub struct Socket {
    src_port: AtomicU32,
    data: Spin<SocketData>,
}

impl Socket {
    pub fn new(port: u32) -> Socket {
        Socket {
            src_port: AtomicU32::new(port),
            data: Spin::new(SocketData::new()),
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
        self.data.lock().close();
    }
}
impl TcpService for Socket {
    fn process_packet(&self, packet: Packet<Tcp>) {
        if let Some(packet) = self.data.lock().process(packet) {
            crate::kernel::net::tcp::send_packet(packet);
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
