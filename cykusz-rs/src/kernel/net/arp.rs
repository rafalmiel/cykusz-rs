use crate::kernel::net::ip::Ip;
use crate::kernel::net::Packet;

pub fn get_dst_mac(mac: &mut [u8], ip: Ip) {
    for v in mac {
        *v = 0xff;
    }
}

pub fn process_packet(packet: Packet) {
    println!("Got ARP packet");
}
