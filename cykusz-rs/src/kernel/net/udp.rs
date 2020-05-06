use crate::kernel::net::ip::{Ip, IpType};
use crate::kernel::net::util::NetU16;
use crate::kernel::net::Packet;

#[repr(packed)]
struct UdpHeader {
    src_port: NetU16,
    dst_port: NetU16,
    len: NetU16,
    crc: NetU16,
}

impl UdpHeader {
    fn set_src_port(&mut self, src: u16) {
        self.src_port.set(src);
    }

    fn set_dst_port(&mut self, dst: u16) {
        self.dst_port.set(dst);
    }

    fn set_len(&mut self, len: u16) {
        self.len.set(len);
    }

    fn compute_checksum(&mut self) {
        self.crc.set(0);
    }
}

impl Packet {
    fn strip_udp_frame(mut self) -> Packet {
        self.addr += core::mem::size_of::<UdpHeader>();

        self
    }

    fn wrap_udp_frame(mut self) -> Packet {
        self.addr -= core::mem::size_of::<UdpHeader>();

        self
    }
}

pub fn create_packet(src_port: u16, dst_port: u16, size: usize, target: Ip) -> Packet {
    let total_len = size + core::mem::size_of::<UdpHeader>();

    let packet = crate::kernel::net::ip::create_packet(IpType::UDP, total_len, target);

    let header = unsafe { packet.addr.read_mut::<UdpHeader>() };

    header.set_dst_port(dst_port);
    header.set_src_port(src_port);
    header.set_len(total_len as u16);

    packet.strip_udp_frame()
}

pub fn send_packet(packet: Packet) {
    let packet = packet.wrap_udp_frame();

    let header = unsafe { packet.addr.read_mut::<UdpHeader>() };
    header.compute_checksum();

    crate::kernel::net::ip::send_packet(packet);
}

pub fn process_packet(packet: Packet) {
    let header = unsafe { packet.addr.read_ref::<UdpHeader>() };

    match header.dst_port.value() {
        68 => crate::kernel::net::dhcp::process_packet(packet.strip_udp_frame()),
        _ => {
            println!("Unknown UDP port {}", header.dst_port.value());
        }
    }
}
