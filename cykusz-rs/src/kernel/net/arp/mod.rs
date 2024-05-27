#![allow(dead_code)]

use core::mem::size_of;

pub use cache::get as cache_get;
pub use cache::insert as cache_insert;
use syscall_defs::net::{NetU16, NetU8};

use crate::kernel::net::eth::{Eth, EthType};
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::{
    default_driver, ConstPacketKind, Packet, PacketDownHierarchy, PacketHeader, PacketUpHierarchy,
};

pub mod cache;

#[derive(Copy, Clone)]
#[repr(u16)]
enum HType {
    Ethernet = NetU16::new(1).net_value(),
    IEEE8023 = NetU16::new(6).net_value(),
    FrameRelay = NetU16::new(15).net_value(),
    ATM = NetU16::new(16).net_value(),
    HDLC = NetU16::new(17).net_value(),
    FibleChannel = NetU16::new(18).net_value(),
    ATM2 = NetU16::new(19).net_value(),
    SerialLine = NetU16::new(20).net_value(),
    ATM3 = NetU16::new(30).net_value(),
    IPsec = NetU16::new(31).net_value(),
}

#[derive(Copy, Clone)]
#[repr(u16)]
enum PType {
    IPv4 = NetU16::new(0x0800).net_value(),
}

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
enum Oper {
    Request = NetU16::new(1).net_value(),
    Reply = NetU16::new(2).net_value(),
    ReverseRequest = NetU16::new(3).net_value(),
    ReverseReply = NetU16::new(4).net_value(),
}

#[repr(packed)]
pub struct ArpHeader {
    htype: HType,
    ptype: PType,
    hlen: NetU8,
    plen: NetU8,
    oper: Oper,
    src_mac: [u8; 6],
    src_ip: Ip4,
    dst_mac: [u8; 6],
    dst_ip: Ip4,
}

#[derive(Debug, Copy, Clone)]
pub struct Arp {}

impl ConstPacketKind for Arp {
    const HSIZE: usize = core::mem::size_of::<ArpHeader>();
}

impl PacketUpHierarchy<Arp> for Packet<Eth> {}

impl PacketHeader<ArpHeader> for Packet<Arp> {}

impl ArpHeader {
    fn init(&mut self) {
        self.htype = HType::Ethernet;
        self.ptype = PType::IPv4;
        self.hlen = NetU8::new(6);
        self.plen = NetU8::new(4);
    }

    fn set_oper(&mut self, oper: Oper) {
        self.oper = oper;
    }

    fn set_src_ip(&mut self, ip: Ip4) {
        self.src_ip = ip;
    }

    fn set_dst_ip(&mut self, ip: Ip4) {
        self.dst_ip = ip;
    }

    fn set_src_mac(&mut self, mac: &[u8]) {
        self.src_mac.copy_from_slice(mac);
    }

    fn set_dst_mac(&mut self, mac: &[u8]) {
        self.dst_mac.copy_from_slice(mac);
    }

    fn oper(&self) -> Oper {
        self.oper
    }

    fn src_ip(&self) -> Ip4 {
        self.src_ip
    }

    fn dst_ip(&self) -> Ip4 {
        self.dst_ip
    }

    fn src_mac(&self) -> &[u8] {
        &self.src_mac
    }

    fn dst_mac(&self) -> &[u8] {
        &self.dst_mac
    }
}

pub fn process_packet(packet: Packet<Arp>) {
    let drv = default_driver();

    let header = packet.header();

    if header.src_ip.is_same_subnet(drv.ip(), drv.subnet()) {
        cache_insert(header.src_ip, &header.src_mac);
    }

    if header.oper() == Oper::Request {
        if header.dst_ip() == drv.ip() {
            let mut packet: Packet<Arp> = crate::kernel::net::eth::create_packet(
                EthType::ARP,
                core::mem::size_of::<ArpHeader>(),
            )
            .upgrade();

            let ohdr = packet.header_mut();

            ohdr.init();
            ohdr.set_oper(Oper::Reply);
            ohdr.set_src_ip(drv.ip());
            ohdr.set_src_mac(&drv.driver.get_mac());
            ohdr.set_dst_ip(header.src_ip());
            ohdr.set_dst_mac(header.src_mac());

            logln_disabled!("[ ARP ] Send reply to {:?}", header.src_ip());

            crate::kernel::net::eth::send_packet(packet.downgrade(), header.src_ip);
        }
    }
}

pub fn request_ip(target: Ip4, to_cache: Packet<Eth>) {
    let mut packet: Packet<Arp> =
        crate::kernel::net::eth::create_packet(EthType::ARP, size_of::<ArpHeader>()).upgrade();

    let drv = default_driver();

    let ohdr = packet.header_mut();

    ohdr.init();
    ohdr.set_oper(Oper::Request);
    ohdr.set_src_ip(drv.ip());
    ohdr.set_src_mac(&drv.driver.get_mac());
    ohdr.set_dst_ip(target);
    ohdr.set_dst_mac(&[0, 0, 0, 0, 0, 0]);

    logln_disabled!("[ ARP ] Send request to {:?}", target);

    cache::request_ip(target, to_cache);

    crate::kernel::net::eth::send_packet(packet.downgrade(), Ip4::limited_broadcast());
}

pub fn init() {
    cache::init();
}
