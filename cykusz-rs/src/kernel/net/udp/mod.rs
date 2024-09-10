use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;

use syscall_defs::net::NetU16;

use crate::kernel::net::ip::{Ip, Ip4, IpHeader, IpType};
use crate::kernel::net::socket::NetSocketService;
use crate::kernel::net::util::checksum;
use crate::kernel::net::{
    ConstPacketKind, Packet, PacketDownHierarchy, PacketHeader, PacketUpHierarchy,
};
use crate::kernel::sync::RwSpin;

pub mod socket;

#[derive(Debug, Copy, Clone)]
pub struct Udp {}

impl ConstPacketKind for Udp {
    const HSIZE: usize = core::mem::size_of::<UdpHeader>();
}

impl PacketUpHierarchy<Udp> for Packet<Ip> {}

impl PacketHeader<UdpHeader> for Packet<Udp> {}

#[repr(C, packed)]
pub struct UdpHeader {
    pub src_port: NetU16,
    pub dst_port: NetU16,
    pub len: NetU16,
    crc: NetU16,
}

impl UdpHeader {
    fn set_src_port(&mut self, src: u16) {
        self.src_port = NetU16::new(src);
    }

    fn set_dst_port(&mut self, dst: u16) {
        self.dst_port = NetU16::new(dst);
    }

    fn set_len(&mut self, len: u16) {
        self.len = NetU16::new(len);
    }

    fn compute_checksum(&mut self, ip: &IpHeader) {
        self.crc = NetU16::new(0);

        self.crc = checksum::make_combine(&[
            checksum::calc_ref(&checksum::PseudoHeader::new(ip)),
            checksum::calc_ref_len(self, self.len.value() as usize),
        ]);
    }
}

pub fn create_packet(src_port: u16, dst_port: u16, size: usize, target: Ip4) -> Packet<Udp> {
    let total_len = size + core::mem::size_of::<UdpHeader>();

    let mut packet: Packet<Udp> =
        crate::kernel::net::ip::create_packet(IpType::UDP, total_len, target).upgrade();

    let header = packet.header_mut();

    header.set_dst_port(dst_port);
    header.set_src_port(src_port);
    header.set_len(total_len as u16);

    packet
}

pub fn send_packet(mut packet: Packet<Udp>) {
    let ip_packet = packet.downgrade();

    let header = packet.header_mut();
    header.compute_checksum(ip_packet.header());

    crate::kernel::net::ip::send_packet(ip_packet);
}

pub fn process_packet(packet: Packet<Udp>) {
    let header = packet.header();

    let tree = HANDLERS.read();

    let dst_port = header.dst_port.value() as u32;

    if let Some(f) = tree.get(&dst_port) {
        let f2 = f.clone();

        drop(tree);

        f2.process_packet(packet.downgrade())
    } else {
        crate::kernel::net::icmp::send_port_unreachable(packet.downgrade());
    }
}

pub fn port_unreachable(port: u32, dst_port: u32) {
    let tree = HANDLERS.read();

    if let Some(f) = tree.get(&port) {
        let f2 = f.clone();

        drop(tree);

        f2.port_unreachable(port, dst_port)
    }
}

static HANDLERS: RwSpin<BTreeMap<u32, Arc<dyn NetSocketService>>> = RwSpin::new(BTreeMap::new());

pub fn register_handler(handler: Arc<dyn NetSocketService>) -> Option<u32> {
    let port = handler.src_port();

    if port == 0 {
        return register_ephemeral_handler(handler);
    }

    let mut handlers = HANDLERS.write();

    if !handlers.contains_key(&port) {
        handlers.insert(port, handler);

        return Some(port);
    }

    None
}

pub fn register_ephemeral_handler(handler: Arc<dyn NetSocketService>) -> Option<u32> {
    let mut handlers = HANDLERS.write();

    for p in 49152..=65535 {
        if !handlers.contains_key(&p) {
            handler.set_src_port(p);

            handlers.insert(p, handler);

            return Some(p);
        }
    }

    None
}

pub fn release_handler(port: u32) {
    let mut handlers = HANDLERS.write();

    if handlers.contains_key(&port) {
        handlers.remove(&port);
    }
}
