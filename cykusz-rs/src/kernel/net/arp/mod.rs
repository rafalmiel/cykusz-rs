#![allow(dead_code)]

use core::mem::size_of;

pub use cache::get as cache_get;
pub use cache::insert as cache_insert;

use crate::kernel::net::eth::EthType;
use crate::kernel::net::ip::Ip;
use crate::kernel::net::util::{NetU16, NetU8};
use crate::kernel::net::{default_driver, Packet};

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
struct ArpHeader {
    htype: HType,
    ptype: PType,
    hlen: NetU8,
    plen: NetU8,
    oper: Oper,
    src_mac: [u8; 6],
    src_ip: Ip,
    dst_mac: [u8; 6],
    dst_ip: Ip,
}

impl Packet {
    fn strip_arp_frame(mut self) -> Packet {
        self.addr += core::mem::size_of::<ArpHeader>();
        self.len -= core::mem::size_of::<ArpHeader>();

        self
    }

    fn wrap_arp_frame(mut self) -> Packet {
        self.addr -= core::mem::size_of::<ArpHeader>();
        self.len += core::mem::size_of::<ArpHeader>();

        self
    }
}

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

    fn set_src_ip(&mut self, ip: Ip) {
        self.src_ip = ip;
    }

    fn set_dst_ip(&mut self, ip: Ip) {
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

    fn src_ip(&self) -> Ip {
        self.src_ip
    }

    fn dst_ip(&self) -> Ip {
        self.dst_ip
    }

    fn src_mac(&self) -> &[u8] {
        &self.src_mac
    }

    fn dst_mac(&self) -> &[u8] {
        &self.dst_mac
    }
}

pub fn process_packet(packet: Packet) {
    let drv = default_driver();

    let header = unsafe { packet.addr.read_ref::<ArpHeader>() };

    if header.src_ip.is_same_subnet(drv.ip(), drv.subnet()) {
        cache_insert(header.src_ip, &header.src_mac);
    }

    if header.oper() == Oper::Request {
        if header.dst_ip() == drv.ip() {
            let packet = crate::kernel::net::eth::create_packet(
                EthType::ARP,
                core::mem::size_of::<ArpHeader>(),
            );

            let ohdr = unsafe { packet.addr.read_mut::<ArpHeader>() };

            ohdr.init();
            ohdr.set_oper(Oper::Reply);
            ohdr.set_src_ip(drv.ip());
            ohdr.set_src_mac(&drv.driver.get_mac());
            ohdr.set_dst_ip(header.src_ip());
            ohdr.set_dst_mac(header.src_mac());

            println!("ARP Send reply to {:?}", header.src_ip());

            crate::kernel::net::eth::send_packet(packet, header.src_ip);
        }
    }
}

pub fn request_ip(target: Ip) {
    let packet = crate::kernel::net::eth::create_packet(EthType::ARP, size_of::<ArpHeader>());

    let drv = default_driver();

    let ohdr = unsafe { packet.addr.read_mut::<ArpHeader>() };

    ohdr.init();
    ohdr.set_oper(Oper::Request);
    ohdr.set_src_ip(drv.ip());
    ohdr.set_src_mac(&drv.driver.get_mac());
    ohdr.set_dst_ip(target);
    ohdr.set_dst_mac(&[0, 0, 0, 0, 0, 0]);

    println!("ARP Send request to {:?}", target);

    crate::kernel::net::eth::send_packet(packet, Ip::limited_broadcast());
}

pub fn init() {
    cache::init();
}
