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

impl EthHeader {
    pub fn src_mac(&self) -> [u8; 6] {
        self.src_mac
    }
}

impl Packet {
    fn strip_eth_frame(mut self) -> Packet {
        self.addr += core::mem::size_of::<EthHeader>();
        self.len -= core::mem::size_of::<EthHeader>();

        self
    }

    fn wrap_eth_frame(mut self) -> Packet {
        self.addr -= core::mem::size_of::<EthHeader>();
        self.len += core::mem::size_of::<EthHeader>();

        self
    }
}

pub fn create_packet(typ: EthType, size: usize, target: Ip) -> Packet {
    use crate::kernel::net::arp::get_dst_mac;

    let drv = crate::kernel::net::default_driver();

    let mut packet = drv
        .driver
        .alloc_packet(size + core::mem::size_of::<EthHeader>());

    let eth = unsafe { packet.addr.read_mut::<EthHeader>() };

    if let Some(mac) = crate::kernel::net::arp::cache_get(target) {
        drv.driver.read_mac(&mut eth.src_mac);
        eth.dst_mac.copy_from_slice(&mac);
        eth.typ = typ;

        packet.strip_eth_frame()
    } else {
        panic!("MAC Addr not found in cache for ip: {:?}", target);
    }
}

pub fn send_packet(packet: Packet) {
    let packet = packet.wrap_eth_frame();

    let drv = crate::kernel::net::default_driver();

    drv.driver.send(packet);
}

pub fn process_packet(packet: Packet) {
    let eth = unsafe { packet.addr.read_ref::<EthHeader>() };

    match eth.typ {
        EthType::IP => crate::kernel::net::ip::process_packet(packet.strip_eth_frame()),
        EthType::ARP => crate::kernel::net::arp::process_packet(packet.strip_eth_frame()),
    }
}
