use crate::kernel::net::ip::{Ip, Ip4, IpType};
use crate::kernel::net::util::NetU16;
use crate::kernel::net::{
    ConstPacketKind, Packet, PacketDownHierarchy, PacketHeader, PacketUpHierarchy,
};

#[derive(Debug, Copy, Clone)]
pub struct Udp {}

impl ConstPacketKind for Udp {
    const HSIZE: usize = core::mem::size_of::<UdpHeader>();
}

impl PacketUpHierarchy<Udp> for Packet<Ip> {}

impl PacketHeader<UdpHeader> for Packet<Udp> {}

#[repr(packed)]
pub struct UdpHeader {
    src_port: NetU16,
    dst_port: NetU16,
    len: NetU16,
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

    fn compute_checksum(&mut self) {
        self.crc = NetU16::new(0);
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
    let header = packet.header_mut();
    header.compute_checksum();

    crate::kernel::net::ip::send_packet(packet.downgrade());
}

pub fn process_packet(packet: Packet<Udp>) {
    let header = packet.header();

    match header.dst_port.value() {
        68 => crate::kernel::net::dhcp::process_packet(packet.upgrade()),
        _ => {
            crate::kernel::net::icmp::send_port_unreachable(packet);
            //println!("Unknown UDP port {}", header.dst_port.value());
        }
    }
}
