#![allow(dead_code)]

use crate::kernel::net::eth::{EthHeader, EthType};
use crate::kernel::net::util::{NetU8, NetU16};
use crate::kernel::net::{default_driver, Packet};

#[repr(u8)]
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
pub struct Ip {
    pub v: [u8; 4],
}

#[derive(Default)]
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
    pub src_ip: Ip,
    pub dest_ip: Ip,
}

impl Ip {
    pub fn limited_broadcast() -> Ip {
        Ip {
            v: [255, 255, 255, 255],
        }
    }

    pub fn empty() -> Ip {
        Ip { v: [0, 0, 0, 0] }
    }

    pub fn is_same_subnet(&self, ip: Ip, subnet: Ip) -> bool {
        for i in 0..4 {
            if !((ip.v[i] & subnet.v[i]) == (self.v[i] & subnet.v[i])) {
                return false;
            }
        }

        return true;
    }
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

    fn set_src_ip(&mut self, ip: Ip) {
        self.src_ip = ip;
    }

    fn set_dest_ip(&mut self, ip: Ip) {
        self.dest_ip = ip;
    }

    fn calc_checksum(&mut self) {
        let mut sum: u32 = 0;

        let ptr = self as *const _ as *const NetU16;

        for i in 0..core::mem::size_of::<IpHeader>() / 2 {
            sum += unsafe { (&*ptr.offset(i as isize)).value() as u32 }
        }

        let mut carry = sum >> 16;
        while carry > 0 {
            sum &= 0x0000_ffff;
            sum += carry;
            carry = sum >> 16;
        }
        self.hcrc = NetU16::new(!(sum as u16));
    }
}

impl Packet {
    fn strip_ip_frame(mut self) -> Packet {
        self.addr += core::mem::size_of::<IpHeader>();
        self.len -= core::mem::size_of::<IpHeader>();

        self
    }

    fn wrap_ip_frame(mut self) -> Packet {
        self.addr -= core::mem::size_of::<IpHeader>();
        self.len += core::mem::size_of::<IpHeader>();

        self
    }

    fn eth_header(&self) -> &EthHeader {
        unsafe { (self.addr - core::mem::size_of::<EthHeader>()).read_ref::<EthHeader>() }
    }
}

pub fn create_packet(typ: IpType, size: usize, target: Ip) -> Packet {
    let total_size = size + core::mem::size_of::<IpHeader>();
    let p = crate::kernel::net::eth::create_packet(EthType::IP, total_size, target);

    let ip = unsafe { p.addr.read_mut::<IpHeader>() };

    ip.init();

    ip.set_length(total_size as u16);
    ip.set_protocol(typ);
    ip.set_dest_ip(target);

    let drv = default_driver();
    ip.set_src_ip(drv.ip());

    ip.calc_checksum();

    p.strip_ip_frame()
}

pub fn send_packet(packet: Packet) {
    let packet = packet.wrap_ip_frame();

    crate::kernel::net::eth::send_packet(packet);
}

pub fn process_packet(packet: Packet) {
    let ip = unsafe { packet.addr.read_ref::<IpHeader>() };

    match ip.protocol {
        IpType::UDP => {
            crate::kernel::net::udp::process_packet(packet.strip_ip_frame());
        }
        IpType::ICMP => crate::kernel::net::icmp::process_packet(packet.strip_ip_frame()),
        _ => {
            println!("Unsupported protocol");
        }
    }
}

pub fn test() {
    let mut ip = IpHeader::default();
    ip.init();

    let v = &ip as *const _ as *const u8;

    for i in 0..16 {
        print!("0x{:x} ", unsafe { v.offset(i).read() });
    }
}
