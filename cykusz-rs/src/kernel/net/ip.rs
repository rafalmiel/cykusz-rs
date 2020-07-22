#![allow(dead_code)]

use core::mem::size_of;

use crate::kernel::net::eth::{Eth, EthHeader, EthType};
use crate::kernel::net::util::{checksum, NetU16, NetU8};
use crate::kernel::net::{
    default_driver, ConstPacketKind, Packet, PacketDownHierarchy, PacketHeader, PacketUpHierarchy,
};

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
#[non_exhaustive]
pub enum IpType {
    ICMP = NetU8::new(1).net_value(),
    TCP = NetU8::new(6).net_value(),
    UDP = NetU8::new(17).net_value(),
}

impl Default for IpType {
    fn default() -> IpType {
        IpType::UDP
    }
}

#[derive(Debug, Default, Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
#[repr(packed)]
pub struct Ip4 {
    pub v: [u8; 4],
}

impl Ip4 {
    pub fn limited_broadcast() -> Ip4 {
        Ip4 {
            v: [255, 255, 255, 255],
        }
    }

    pub fn new(d: &[u8]) -> Ip4 {
        if d.len() < 4 {
            panic!("Invalid Ip4 array")
        } else {
            Ip4 {
                v: [d[0], d[1], d[2], d[3]],
            }
        }
    }

    pub fn empty() -> Ip4 {
        Ip4 { v: [0, 0, 0, 0] }
    }

    pub fn is_same_subnet(&self, ip: Ip4, subnet: Ip4) -> bool {
        for i in 0..4 {
            if !((ip.v[i] & subnet.v[i]) == (self.v[i] & subnet.v[i])) {
                return false;
            }
        }

        return true;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Ip {}

impl ConstPacketKind for Ip {
    const HSIZE: usize = core::mem::size_of::<IpHeader>();
}

impl PacketUpHierarchy<Ip> for Packet<Eth> {}

impl PacketHeader<IpHeader> for Packet<Ip> {}

impl Packet<Ip> {
    fn eth_header(&self) -> &EthHeader {
        unsafe { (self.addr - size_of::<EthHeader>()).read_ref::<EthHeader>() }
    }
}

#[derive(Debug, Default, Copy, Clone)]
#[repr(packed)]
pub struct IpHeader {
    pub v: NetU8,
    pub tos: NetU8,
    pub len: NetU16,
    pub ident: NetU16,
    pub frag_offset: NetU16,
    pub ttl: NetU8,
    pub protocol: IpType,
    pub hcrc: NetU16,
    pub src_ip: Ip4,
    pub dest_ip: Ip4,
}

impl IpHeader {
    fn init(&mut self) {
        self.v = NetU8::new(0x45);
        self.tos = NetU8::new(0);
        self.ident = NetU16::new(0);
        self.frag_offset = NetU16::new(0);
        self.ttl = NetU8::new(64);
        self.hcrc = NetU16::new(0);
    }

    fn set_length(&mut self, len: u16) {
        self.len = NetU16::new(len);
    }

    fn set_protocol(&mut self, p: IpType) {
        self.protocol = p;
    }

    fn set_src_ip(&mut self, ip: Ip4) {
        self.src_ip = ip;
    }

    fn set_dest_ip(&mut self, ip: Ip4) {
        self.dest_ip = ip;
    }

    pub fn calc_checksum(&mut self) {
        self.hcrc = NetU16::new(0);
        self.hcrc = checksum::make(checksum::calc_ref(self));
    }
}

pub fn create_packet(typ: IpType, size: usize, target: Ip4) -> Packet<Ip> {
    let total_size = size + core::mem::size_of::<IpHeader>();

    let mut p: Packet<Ip> =
        crate::kernel::net::eth::create_packet(EthType::IP, total_size).upgrade();

    let ip = p.header_mut();

    ip.init();

    ip.set_length(total_size as u16);
    ip.set_protocol(typ);
    ip.set_dest_ip(target);

    let drv = default_driver();
    ip.set_src_ip(drv.ip());

    ip.calc_checksum();

    p
}

pub fn send_packet(packet: Packet<Ip>) {
    let ip = packet.header();

    crate::kernel::net::eth::send_packet(packet.downgrade(), ip.dest_ip);
}

pub fn process_packet(packet: Packet<Ip>) {
    let ip = packet.header();

    #[allow(unreachable_patterns)]
    match ip.protocol {
        IpType::UDP => crate::kernel::net::udp::process_packet(packet.upgrade()),
        IpType::TCP => crate::kernel::net::tcp::process_packet(packet.upgrade()),
        IpType::ICMP => crate::kernel::net::icmp::process_packet(packet.upgrade()),
        _ => {
            println!("Unsupported protocol");
        }
    }
}
