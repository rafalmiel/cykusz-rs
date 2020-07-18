use core::mem::size_of;

use crate::kernel::net::ip::{Ip, IpHeader, IpType};
use crate::kernel::net::udp::{Udp, UdpHeader};
use crate::kernel::net::util::NetU16;
use crate::kernel::net::{
    Packet, PacketBaseTrait, PacketDownHierarchy, PacketHeader, PacketKind, PacketUpHierarchy,
};

#[derive(Debug, Copy, Clone)]
pub struct Icmp {}

impl PacketKind for Icmp {}

impl PacketUpHierarchy<Icmp> for Packet<Ip> {}

impl PacketHeader<IcmpHeader> for Packet<Icmp> {}

#[derive(Eq, PartialEq)]
#[repr(u8)]
enum IcmpType {
    EchoRequest = 8,
    EchoReply = 0,
    DestUnreachable = 3,
}

#[repr(packed)]
struct IcmpHeader {
    typ: IcmpType,
    code: u8,
    crc: NetU16,
}

impl Packet<Icmp> {
    fn ip_header(&self) -> &IpHeader {
        unsafe { (self.addr - size_of::<IpHeader>()).read_ref::<IpHeader>() }
    }

    fn icmp_echo_header(&self) -> &IcmpEchoHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_ref::<IcmpEchoHeader>() }
    }

    fn icmp_echo_data(&self) -> &[u8] {
        let hdr_len = size_of::<IcmpHeader>() + size_of::<IcmpEchoHeader>();

        unsafe {
            core::slice::from_raw_parts((self.addr + hdr_len).0 as *const u8, self.len() - hdr_len)
        }
    }

    fn icmp_dest_unreachable_header(&self) -> &IcmpDestUnreachableHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_ref::<IcmpDestUnreachableHeader>() }
    }

    fn icmp_echo_header_mut(&mut self) -> &mut IcmpEchoHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_mut::<IcmpEchoHeader>() }
    }

    fn icmp_echo_data_mut(&mut self) -> &mut [u8] {
        let hdr_len = size_of::<IcmpHeader>() + size_of::<IcmpEchoHeader>();

        unsafe {
            core::slice::from_raw_parts_mut(
                (self.addr + hdr_len).0 as *mut u8,
                self.len() - hdr_len,
            )
        }
    }

    fn icmp_dest_unreachable_header_mut(&self) -> &mut IcmpDestUnreachableHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_mut::<IcmpDestUnreachableHeader>() }
    }
}

impl IcmpHeader {
    fn calc_checksum(&mut self, len: usize) {
        self.crc = NetU16::new(0);
        let mut sum: u32 = 0;

        let ptr = self as *const _ as *const NetU16;

        if len % 2 == 1 {
            panic!("FIX CHECKSUM");
        }

        for i in 0..(len / 2) {
            sum += unsafe { (&*ptr.offset(i as isize)).value() as u32 }
        }

        let mut carry = sum >> 16;
        while carry > 0 {
            sum &= 0x0000_ffff;
            sum += carry;
            carry = sum >> 16;
        }
        self.crc = NetU16::new(!(sum as u16));
    }
}

#[repr(packed)]
struct IcmpEchoHeader {
    echo_id: NetU16,
    echo_seq: NetU16,
}

struct IcmpDestUnreachableHeader {
    empty: NetU16,
    next_mtu: NetU16,
    iphdr: IpHeader,
    orig_payload: [u8; 8],
}

fn process_echo(packet: Packet<Icmp>) {
    let mut out_packet: Packet<Icmp> = crate::kernel::net::ip::create_packet(
        IpType::ICMP,
        packet.len(),
        packet.ip_header().src_ip,
    )
    .upgrade();

    {
        let icmp_hdr = out_packet.header_mut();
        icmp_hdr.typ = IcmpType::EchoReply;
        icmp_hdr.code = 0;
    }

    {
        let icmp_echo_hdr = out_packet.icmp_echo_header_mut();
        let src_echo_hdr = packet.icmp_echo_header();

        icmp_echo_hdr.echo_id = src_echo_hdr.echo_id;
        icmp_echo_hdr.echo_seq = src_echo_hdr.echo_seq;
    }

    {
        let data = out_packet.icmp_echo_data_mut();
        let src_data = packet.icmp_echo_data();

        data.copy_from_slice(src_data);
    }

    {
        let icmp_hdr = out_packet.header_mut();
        icmp_hdr.calc_checksum(packet.len());
    }

    println!("[ ICMP ] Sending Echo Reply");

    crate::kernel::net::ip::send_packet(out_packet.downgrade());
}

pub fn process_dest_unreachable(packet: Packet<Icmp>) {
    let hdr = packet.icmp_dest_unreachable_header();

    match hdr.iphdr.protocol {
        IpType::UDP => {
            let udp = unsafe { &*(hdr.orig_payload.as_ptr() as *const UdpHeader) };

            println!(
                "[ ICMP ] Dest unreachable {} {}",
                udp.src_port.value(),
                udp.dst_port.value()
            );
            crate::kernel::net::udp::port_unreachable(
                udp.src_port.value() as u32,
                udp.dst_port.value() as u32,
            );
        }
        _ => {}
    }
}

pub fn process_packet(packet: Packet<Icmp>) {
    let hdr = packet.header();

    match hdr.typ {
        IcmpType::EchoRequest => {
            process_echo(packet);
        }
        IcmpType::DestUnreachable => {
            process_dest_unreachable(packet);
        }
        IcmpType::EchoReply => {}
    }
}

pub fn send_port_unreachable(udp_packet: Packet<Udp>) {
    let orig_ip = unsafe { (udp_packet.addr - size_of::<IpHeader>()).read_ref::<IpHeader>() };

    let payload_len = size_of::<IcmpHeader>() + size_of::<IcmpDestUnreachableHeader>();

    let mut out_packet: Packet<Icmp> =
        crate::kernel::net::ip::create_packet(IpType::ICMP, payload_len, orig_ip.src_ip).upgrade();

    {
        let hdr = out_packet.header_mut();
        hdr.typ = IcmpType::DestUnreachable;
        hdr.code = 3;
    }

    {
        let hdr = out_packet.icmp_dest_unreachable_header_mut();
        hdr.empty = NetU16::new(0);
        hdr.next_mtu = NetU16::new(0);
        hdr.iphdr = *orig_ip;
        hdr.orig_payload.copy_from_slice(unsafe {
            core::slice::from_raw_parts((udp_packet.addr).0 as *mut u8, 8)
        });
    }

    {
        let hdr = out_packet.header_mut();
        hdr.calc_checksum(payload_len);
    }

    crate::kernel::net::ip::send_packet(out_packet.downgrade());
}
