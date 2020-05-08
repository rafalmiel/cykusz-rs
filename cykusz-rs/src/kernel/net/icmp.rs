use core::mem::size_of;

use crate::kernel::net::ip::{IpHeader, IpType};
use crate::kernel::net::util::NetU16;
use crate::kernel::net::Packet;

#[derive(Eq, PartialEq)]
#[repr(u8)]
enum IcmpType {
    EchoRequest = 8,
    EchoReply = 0,
}

#[repr(packed)]
struct IcmpHeader {
    typ: IcmpType,
    code: u8,
    crc: NetU16,
}

impl Packet {
    fn ip_header(&self) -> &IpHeader {
        unsafe { (self.addr - size_of::<IpHeader>()).read_ref::<IpHeader>() }
    }

    fn icmp_header(&self) -> &IcmpHeader {
        unsafe { self.addr.read_ref::<IcmpHeader>() }
    }

    fn icmp_echo_header(&self) -> &IcmpEchoHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_ref::<IcmpEchoHeader>() }
    }

    fn icmp_echo_data(&self) -> &[u8] {
        let hdr_len = size_of::<IcmpHeader>() + size_of::<IcmpEchoHeader>();

        unsafe {
            core::slice::from_raw_parts((self.addr + hdr_len).0 as *const u8, self.len - hdr_len)
        }
    }

    fn icmp_header_mut(&mut self) -> &mut IcmpHeader {
        unsafe { self.addr.read_mut::<IcmpHeader>() }
    }

    fn icmp_echo_header_mut(&mut self) -> &mut IcmpEchoHeader {
        unsafe { (self.addr + size_of::<IcmpHeader>()).read_mut::<IcmpEchoHeader>() }
    }

    fn icmp_echo_data_mut(&mut self) -> &mut [u8] {
        let hdr_len = size_of::<IcmpHeader>() + size_of::<IcmpEchoHeader>();

        unsafe {
            core::slice::from_raw_parts_mut((self.addr + hdr_len).0 as *mut u8, self.len - hdr_len)
        }
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

pub fn process_packet(packet: Packet) {
    let hdr = packet.icmp_header();

    if hdr.typ == IcmpType::EchoRequest {
        let mut out_packet = crate::kernel::net::ip::create_packet(
            IpType::ICMP,
            packet.len,
            packet.ip_header().src_ip,
        );

        {
            let icmp_hdr = out_packet.icmp_header_mut();
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
            let icmp_hdr = out_packet.icmp_header_mut();
            icmp_hdr.calc_checksum(packet.len);
        }

        println!("[ ICMP ] Sending Echo Reply");

        crate::kernel::net::ip::send_packet(out_packet);
    }
}
