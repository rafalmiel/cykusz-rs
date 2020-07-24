use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use bit_field::BitField;

use crate::kernel::net::ip::{Ip, Ip4, IpHeader, IpType};
use crate::kernel::net::util::{checksum, NetU16, NetU32};
use crate::kernel::net::{
    Packet, PacketDownHierarchy, PacketHeader, PacketKind, PacketUpHierarchy,
};
use crate::kernel::sync::RwSpin;

pub mod socket;

#[derive(Debug, Copy, Clone)]
pub struct Tcp {}

impl PacketKind for Tcp {}

impl PacketUpHierarchy<Tcp> for Packet<Ip> {}

impl PacketHeader<TcpHeader> for Packet<Tcp> {}

#[repr(packed)]
pub struct TcpHeader {
    src_port: NetU16,
    dst_port: NetU16,
    seq_nr: NetU32,
    ack_nr: NetU32,
    flags: NetU16,
    window: NetU16,
    checksum: NetU16,
    urgent_ptr: NetU16,
}

impl TcpHeader {
    pub fn src_port(&self) -> u16 {
        self.src_port.value()
    }

    pub fn set_src_port(&mut self, val: u16) {
        self.src_port = NetU16::new(val);
    }

    pub fn dst_port(&self) -> u16 {
        self.dst_port.value()
    }

    pub fn set_dst_port(&mut self, val: u16) {
        self.dst_port = NetU16::new(val)
    }

    pub fn seq_nr(&self) -> u32 {
        self.seq_nr.value()
    }

    pub fn set_seq_nr(&mut self, val: u32) {
        self.seq_nr = NetU32::new(val)
    }

    pub fn ack_nr(&self) -> u32 {
        self.ack_nr.value()
    }

    pub fn set_ack_nr(&mut self, val: u32) {
        self.ack_nr = NetU32::new(val);
        self.set_flag_ack(true);
    }

    pub fn flags(&self) -> u16 {
        self.flags.value()
    }

    pub fn header_len(&self) -> u8 {
        (self.flags.value().get_bits(12..=15) * 4) as u8
    }

    pub fn set_header_len(&mut self, len: u8) {
        let mut f = self.flags.value();

        f.set_bits(12..=15, len as u16 / 4);

        self.flags = NetU16::new(f);
    }

    fn set_flag(&mut self, idx: usize, flag: bool) {
        let mut f = self.flags.value();

        f.set_bit(idx, flag);

        self.flags = NetU16::new(f);
    }

    pub fn flag_fin(&self) -> bool {
        self.flags.value().get_bit(0)
    }

    pub fn set_flag_fin(&mut self, flag: bool) {
        self.set_flag(0, flag)
    }

    pub fn flag_syn(&self) -> bool {
        self.flags.value().get_bit(1)
    }

    pub fn set_flag_syn(&mut self, flag: bool) {
        self.set_flag(1, flag)
    }

    pub fn flag_rst(&self) -> bool {
        self.flags.value().get_bit(2)
    }

    pub fn set_flag_rst(&mut self, flag: bool) {
        self.set_flag(2, flag);
    }

    pub fn flag_psh(&self) -> bool {
        self.flags.value().get_bit(3)
    }

    pub fn set_flag_psh(&mut self, flag: bool) {
        self.set_flag(3, flag);
    }

    pub fn flag_ack(&self) -> bool {
        self.flags.value().get_bit(4)
    }

    pub fn set_flag_ack(&mut self, flag: bool) {
        self.set_flag(4, flag);
    }

    pub fn flag_urg(&self) -> bool {
        self.flags.value().get_bit(5)
    }

    pub fn set_flag_urg(&mut self, flag: bool) {
        self.set_flag(5, flag);
    }

    pub fn window(&self) -> u16 {
        self.window.value()
    }

    pub fn set_window(&mut self, val: u16) {
        self.window = NetU16::new(val)
    }

    pub fn checksum(&self) -> u16 {
        self.checksum.value()
    }

    pub fn set_checksum(&mut self, val: u16) {
        self.checksum = NetU16::new(val)
    }

    pub fn urgent_ptr(&self) -> u16 {
        self.urgent_ptr.value()
    }

    pub fn set_urgent_ptr(&mut self, val: u16) {
        self.urgent_ptr = NetU16::new(val)
    }

    pub fn calc_checksum(&mut self, ip: &IpHeader) {
        self.checksum = NetU16::new(0);

        self.checksum = checksum::make_combine(&[
            checksum::calc_ref(&checksum::PseudoHeader::new(ip)),
            checksum::calc_ref_len(
                self,
                ip.len.value() as usize - core::mem::size_of::<IpHeader>(),
            ),
        ]);
    }
}

pub fn create_packet(src_port: u16, dst_port: u16, size: usize, target: Ip4) -> Packet<Tcp> {
    let total_len = size + core::mem::size_of::<TcpHeader>();

    let mut packet: Packet<Tcp> =
        crate::kernel::net::ip::create_packet(IpType::TCP, total_len, target).upgrade();

    let header = packet.header_mut();

    header.set_dst_port(dst_port);
    header.set_src_port(src_port);
    header.set_header_len(core::mem::size_of::<TcpHeader>() as u8);

    packet
}

pub fn send_packet(mut packet: Packet<Tcp>) {
    let ip_packet = packet.downgrade();

    let header = packet.header_mut();
    header.calc_checksum(ip_packet.header());

    crate::kernel::net::ip::send_packet(ip_packet);
}

pub fn process_packet(packet: Packet<Tcp>) {
    let header = packet.header();

    let mut tree = HANDLERS.write();

    let dst_port = header.dst_port.value() as u32;

    if let Some(f) = tree.get_mut(&dst_port) {
        let f = f.clone();

        drop(tree);

        f.process_packet(packet);
    } else {
        crate::kernel::net::icmp::send_port_unreachable(packet.downgrade());
    }
}

pub trait TcpService: Sync + Send {
    fn process_packet(&self, packet: Packet<Tcp>);
    fn port_unreachable(&self, port: u32, dst_port: u32);
}

static HANDLERS: RwSpin<BTreeMap<u32, Arc<dyn TcpService>>> = RwSpin::new(BTreeMap::new());

pub fn register_handler(port: u32, handler: Arc<dyn TcpService>) -> bool {
    let mut handlers = HANDLERS.write();

    if !handlers.contains_key(&port) {
        handlers.insert(port, handler);

        return true;
    }

    false
}

pub fn register_ephemeral_handler(handler: Arc<dyn TcpService>) -> Option<u32> {
    let mut handlers = HANDLERS.write();

    for p in 49152..=65535 {
        if !handlers.contains_key(&p) {
            handlers.insert(p, handler);

            return Some(p);
        }
    }

    None
}

pub fn release_handler(port: u32) {
    let mut handlers = HANDLERS.write();

    if handlers.contains_key(&port) {
        handlers.remove(&port);
    } else {
        panic!("TCP port is not registered")
    }
}
