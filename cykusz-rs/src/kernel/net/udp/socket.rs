use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use syscall_defs::OpenFlags;
use syscall_defs::poll::PollEventFlags;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::net::ip::{Ip4, IpHeader};
use crate::kernel::net::udp::{Udp, UdpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader, PacketTrait};
use crate::kernel::sync::Spin;
use crate::kernel::utils::buffer::BufferQueue;

pub struct Socket {
    buffer: BufferQueue,
    src_port: AtomicU32,
    dst_port: Spin<Option<u32>>,
    dst_ip: Spin<Option<Ip4>>,
    error: AtomicBool,
}

impl Socket {
    pub fn new(port: u32) -> Socket {
        Socket {
            buffer: BufferQueue::new(4096 * 4),
            src_port: AtomicU32::new(port),
            dst_port: Spin::new(None),
            dst_ip: Spin::new(None),
            error: AtomicBool::new(false),
        }
    }

    pub fn set_dst_port(&self, port: u32) {
        *self.dst_port.lock() = Some(port);
    }

    pub fn dst_port(&self) -> Option<u32> {
        *self.dst_port.lock()
    }

    pub fn set_src_port(&self, port: u32) {
        self.src_port.store(port, Ordering::SeqCst);
    }

    pub fn src_port(&self) -> u32 {
        self.src_port.load(Ordering::SeqCst)
    }

    pub fn set_dst_ip(&self, ip: Ip4) {
        *self.dst_ip.lock() = Some(ip)
    }

    pub fn dst_ip(&self) -> Option<Ip4> {
        *self.dst_ip.lock()
    }

    pub fn error(&self) -> bool {
        self.error.load(Ordering::SeqCst)
    }

    pub fn set_error(&self, e: bool) {
        self.error.store(e, Ordering::SeqCst);
    }
}

impl INode for Socket {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buffer.read_data(buf)?)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        if let (Some(dst_port), Some(dst_ip)) = (self.dst_port(), self.dst_ip()) {
            let mut packet = crate::kernel::net::udp::create_packet(
                self.src_port() as u16,
                dst_port as u16,
                buf.len(),
                dst_ip,
            );

            let dest_buf = packet.data_mut();

            let amount = core::cmp::min(dest_buf.len(), buf.len());

            dest_buf[..amount].copy_from_slice(&buf[..amount]);

            crate::kernel::net::udp::send_packet(packet);

            Ok(amount)
        } else {
            Err(FsError::NotSupported)
        }
    }

    fn poll(&self, listen: Option<&mut PollTable>, flags: PollEventFlags) -> Result<PollEventFlags> {
        if self.error() || !flags.contains(PollEventFlags::READ) {
            return Err(FsError::NotSupported);
        }

        let has_data = self.buffer.has_data();

        if let Some(p) = listen {
            p.listen(self.buffer.readers_queue());
        }

        Ok(if has_data {
            PollEventFlags::READ
        } else {
            PollEventFlags::empty()
        })
    }

    fn close(&self, _flags: OpenFlags) {
        crate::kernel::net::udp::release_handler(self.src_port());
    }
}

impl UdpService for Socket {
    fn process_packet(&self, packet: Packet<Udp>) {
        let header = packet.header();
        let ip = packet.downgrade();
        let ip_header: &IpHeader = ip.header();

        self.set_dst_port(header.src_port.value() as u32);
        self.set_dst_ip(ip_header.src_ip);

        self.buffer.try_append_data(packet.data());
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);

        self.set_error(true);
        self.buffer.readers_queue().notify_all();
    }
}

pub fn bind(port: u32) -> Option<Arc<dyn INode>> {
    let socket = Arc::new(Socket::new(port));

    if crate::kernel::net::udp::register_handler(port, socket.clone()) {
        Some(socket)
    } else {
        None
    }
}

pub fn connect(host: Ip4, port: u32) -> Option<Arc<dyn INode>> {
    let socket = Arc::new(Socket::new(0));

    if let Some(p) = crate::kernel::net::udp::register_ephemeral_handler(socket.clone()) {
        socket.set_src_port(p);
        socket.set_dst_ip(host);
        socket.set_dst_port(port);

        Some(socket)
    } else {
        None
    }
}
