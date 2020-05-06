#![allow(dead_code)]

use crate::kernel::net::ip::Ip;
use crate::kernel::net::util::NetU16;
use crate::kernel::net::{Packet, RecvPacket};

#[repr(u16)]
pub enum EthType {
    IP = NetU16::new(0x0800).net_value(),
    ARP = NetU16::new(0x0806).net_value(),
}

#[repr(packed)]
pub struct EthHeader {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    typ: EthType,
}

impl Packet {
    fn strip_eth_frame(mut self) -> Packet {
        self.addr += core::mem::size_of::<EthHeader>();

        self
    }

    fn wrap_eth_frame(mut self) -> Packet {
        self.addr -= core::mem::size_of::<EthHeader>();

        self
    }
}

pub fn create_packet(typ: EthType, size: usize, target: Ip) -> Packet {
    use crate::kernel::net::arp::get_dst_mac;

    let drv = crate::kernel::net::default_driver();

    let mut packet = drv.alloc_packet(size + core::mem::size_of::<EthHeader>());

    let eth = unsafe { packet.addr.read_mut::<EthHeader>() };

    drv.get_mac(&mut eth.src_mac);
    get_dst_mac(&mut eth.dst_mac, target);
    eth.typ = typ;

    packet.strip_eth_frame()
}

pub fn send_packet(packet: Packet) {
    let packet = packet.wrap_eth_frame();

    let drv = crate::kernel::net::default_driver();

    println!("Sending {} {}", packet.addr, packet.len);

    drv.send(packet);
}

pub fn process_packet(packet: Packet) {
    let eth = unsafe { packet.addr.read_ref::<EthHeader>() };

    match eth.typ {
        EthType::IP => crate::kernel::net::ip::process_packet(packet.strip_eth_frame()),
        EthType::ARP => crate::kernel::net::arp::process_packet(packet.strip_eth_frame()),
    }
}
