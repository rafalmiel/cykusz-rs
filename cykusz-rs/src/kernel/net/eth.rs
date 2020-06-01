#![allow(dead_code)]

use crate::kernel::net::ip::Ip4;
use crate::kernel::net::util::NetU16;
use crate::kernel::net::{ConstPacketKind, Packet};
use crate::kernel::net::{PacketHeader, PacketUpHierarchy};

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

#[derive(Debug, Copy, Clone)]
pub struct Eth {}

impl ConstPacketKind for Eth {
    const HSIZE: usize = core::mem::size_of::<EthHeader>();
}

impl PacketHeader<EthHeader> for Packet<Eth> {}

pub fn create_packet(typ: EthType, size: usize) -> Packet<Eth> {
    let drv = crate::kernel::net::default_driver();

    let mut packet = drv
        .driver
        .alloc_packet(size + core::mem::size_of::<EthHeader>());

    let eth = packet.header_mut();

    drv.driver.read_mac(&mut eth.src_mac);
    eth.typ = typ;
    packet
}

pub fn send_packet(mut packet: Packet<Eth>, target: Ip4) {
    let eth = packet.header_mut();

    let drv = crate::kernel::net::default_driver();

    if target == Ip4::limited_broadcast() || drv.ip().is_same_subnet(target, drv.subnet()) {
        if let Some(mac) = crate::kernel::net::arp::cache_get(target) {
            eth.dst_mac.copy_from_slice(&mac);
            drv.driver.send(packet);
        } else {
            crate::kernel::net::arp::request_ip(target, packet);
        }
    } else {
        eth.dst_mac.copy_from_slice(&drv.driver.get_mac());
        drv.driver.send(packet);
    }
}

pub fn send_packet_to_mac(mut packet: Packet<Eth>, mac: &[u8; 6]) {
    let drv = crate::kernel::net::default_driver();

    let eth = packet.header_mut();

    eth.dst_mac.copy_from_slice(mac);
    drv.driver.send(packet);
}

pub fn process_packet(packet: Packet<Eth>) {
    let eth = packet.header();

    match eth.typ {
        EthType::IP => crate::kernel::net::ip::process_packet(packet.upgrade()),
        EthType::ARP => crate::kernel::net::arp::process_packet(packet.upgrade()),
    }
}
