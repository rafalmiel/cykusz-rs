#![allow(dead_code)]

use crate::kernel::net::ip::Ip;
use crate::kernel::net::util::NetU16;
use crate::kernel::net::Packet;

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

pub fn create_packet(typ: EthType, size: usize) -> Packet {
    let drv = crate::kernel::net::default_driver();

    let packet = drv
        .driver
        .alloc_packet(size + core::mem::size_of::<EthHeader>());

    let eth = unsafe { packet.addr.read_mut::<EthHeader>() };

    drv.driver.read_mac(&mut eth.src_mac);
    eth.typ = typ;
    packet.strip_eth_frame()
}

pub fn send_packet(packet: Packet, target: Ip) {
    let packet = packet.wrap_eth_frame();

    let eth = unsafe { packet.addr.read_mut::<EthHeader>() };

    let drv = crate::kernel::net::default_driver();

    if let Some(mac) = crate::kernel::net::arp::cache_get(target) {
        eth.dst_mac.copy_from_slice(&mac);
        drv.driver.send(packet);
    } else {
        crate::kernel::net::arp::request_ip(target);
    }
}

pub fn send_packet_to_mac(packet: Packet, mac: &[u8; 6]) {
    let drv = crate::kernel::net::default_driver();

    let eth = unsafe { packet.addr.read_mut::<EthHeader>() };

    eth.dst_mac.copy_from_slice(mac);
    drv.driver.send(packet);
}

pub fn process_packet(packet: Packet) {
    let eth = unsafe { packet.addr.read_ref::<EthHeader>() };

    match eth.typ {
        EthType::IP => crate::kernel::net::ip::process_packet(packet.strip_eth_frame()),
        EthType::ARP => crate::kernel::net::arp::process_packet(packet.strip_eth_frame()),
    }
}
