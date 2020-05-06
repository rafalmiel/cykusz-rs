#![allow(dead_code)]

use core::marker::PhantomData;

use crate::kernel::net::ip::Ip;
use crate::kernel::net::util::{NetU16, NetU32, NetU8};
use crate::kernel::net::{default_driver, Packet};

const DHCP_XID: u32 = 0x43424140;

#[repr(u8)]
enum DhcpType {
    BootRequest = NetU8::new(1).net_value(),
    BootReply = NetU8::new(2).net_value(),
}

#[repr(u8)]
enum HType {
    Ethernet = NetU8::new(1).net_value(),
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum DhcpOptMsgType {
    DhcpDiscover = NetU8::new(1).net_value(),
    DhcpOffer = NetU8::new(2).net_value(),
    DhcpRequest = NetU8::new(3).net_value(),
    DhcpDecline = NetU8::new(4).net_value(),
    DhcpAck = NetU8::new(5).net_value(),
    DhcpNAck = NetU8::new(6).net_value(),
    DhcpRelease = NetU8::new(7).net_value(),
}

#[repr(packed)]
struct DhcpHeader {
    op: DhcpType,
    htype: HType,
    hlen: NetU8,
    hops: NetU8,
    xid: NetU32,
    seconds: NetU16,
    flags: NetU16,
    client_ip: Ip,
    your_ip: Ip,
    server_ip: Ip,
    gateway_ip: Ip,
    client_hw_addr: [u8; 16],
    server_name: [u8; 64],
    file: [u8; 128],
    options: [u8; 64],
}

impl DhcpHeader {
    fn init(&mut self) {
        self.htype = HType::Ethernet;
        self.hlen.set(6);
        self.hops.set(0);
        self.xid.set(DHCP_XID);
        self.seconds.set(0);
        self.init_hw_addr();
        self.server_name.fill(0);
        self.file.fill(0);
        self.options.fill(0);
    }

    fn init_hw_addr(&mut self) {
        let d = default_driver();
        d.get_mac(&mut self.client_hw_addr[0..6]);
        self.client_hw_addr[6..].fill(0);
    }

    fn set_op(&mut self, op: DhcpType) {
        self.op = op;
    }

    fn set_flags_broadcast(&mut self, broadcast: bool) {
        if broadcast {
            self.flags.set(0x8000);
        } else {
            self.flags.set(0);
        }
    }

    fn set_client_ip(&mut self, ip: Ip) {
        self.client_ip = ip;
    }

    fn set_your_ip(&mut self, ip: Ip) {
        self.your_ip = ip;
    }

    fn your_ip(&self) -> Ip {
        self.your_ip
    }

    fn set_server_ip(&mut self, ip: Ip) {
        self.server_ip = ip;
    }

    fn set_gateway_ip(&mut self, ip: Ip) {
        self.gateway_ip = ip
    }

    fn options_builder(&mut self) -> OptionsBuilder {
        OptionsBuilder {
            ptr: self.options.as_mut_ptr(),
            len: self.options.len(),
        }
    }

    fn iter<'a>(&self) -> OptionsIter<'a> {
        OptionsIter {
            ptr: unsafe { self.options.as_ptr().offset(4) },
            len: self.options.len(),
            _ph: PhantomData::default(),
        }
    }

    fn opt_message_type(&self) -> Option<DhcpOptMsgType> {
        self.iter()
            .find(|p| return (*p).0 == 53)
            .and_then(|p| unsafe { Some(*(p.1.as_ptr() as *const DhcpOptMsgType)) })
    }
}

struct OptionsBuilder {
    ptr: *mut u8,
    len: usize,
}

struct OptionsIter<'a> {
    ptr: *const u8,
    len: usize,
    _ph: PhantomData<&'a u8>,
}

impl OptionsBuilder {
    fn shift(mut self, by: isize) -> OptionsBuilder {
        assert!(self.len >= by as usize);

        self.ptr = unsafe { self.ptr.offset(by) };
        self.len -= by as usize;

        self
    }

    fn set_magic_cookie(mut self) -> OptionsBuilder {
        unsafe {
            (self.ptr as *mut u32).write(NetU32::new(0x63825363).net_value());
        }

        self.shift(4)
    }

    fn set_message_type(mut self, typ: DhcpOptMsgType) -> OptionsBuilder {
        unsafe {
            self.ptr.offset(0).write(53);
            self.ptr.offset(1).write(1);
            self.ptr.offset(2).write(typ as u8);
        }

        self.shift(3)
    }

    fn set_client_identifier(mut self) -> OptionsBuilder {
        let drv = default_driver();
        unsafe {
            self.ptr.offset(0).write(61);
            self.ptr.offset(1).write(7);
            self.ptr.offset(2).write(HType::Ethernet as u8);

            let sl = core::slice::from_raw_parts_mut(self.ptr.offset(3), 6);

            drv.get_mac(sl);
        }

        self.shift(9)
    }

    fn set_requested_ip(mut self, ip: Ip) -> OptionsBuilder {
        unsafe {
            self.ptr.offset(0).write(50);
            self.ptr.offset(1).write(4);
            self.ptr.offset(2).copy_from(ip.v.as_ptr(), 4);
        }

        self.shift(6)
    }

    fn set_host_name(mut self, name: &str) -> OptionsBuilder {
        let len = name.len();

        unsafe {
            self.ptr.offset(0).write(12);
            self.ptr.offset(1).write(len as u8);
            self.ptr.offset(2).copy_from(name.as_ptr(), len);
            self.ptr.offset(2 + len as isize).write(0);
        }

        self.shift(len as isize + 3)
    }

    fn set_parameter_request_list(mut self) -> OptionsBuilder {
        unsafe {
            self.ptr.offset(0).write(55);
            self.ptr.offset(1).write(4);
            self.ptr.offset(2).write(1); // Subnet Mask
            self.ptr.offset(3).write(3); // Router
            self.ptr.offset(4).write(15); // Domain Name
            self.ptr.offset(5).write(6); // Domain Name Server
        }

        self.shift(6)
    }

    fn finish(mut self) {
        unsafe {
            self.ptr.offset(0).write(0xff);
            self.ptr.offset(1).write_bytes(0, self.len - 1);
        }
    }
}

impl<'a> OptionsIter<'a> {
    fn shift_inplace(&mut self, by: isize) {
        assert!(self.len >= by as usize);

        self.ptr = unsafe { self.ptr.offset(by) };
        self.len -= by as usize;
    }
}

impl<'a> Iterator for OptionsIter<'a> {
    type Item = (u8, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            match unsafe { self.ptr.read() } {
                0 => {
                    self.shift_inplace(1);
                    return self.next();
                }
                255 => {
                    return None;
                }
                o => {
                    let len = unsafe { self.ptr.offset(1).read() };

                    let ptr = unsafe { self.ptr.offset(2) };

                    self.shift_inplace(2 + len as isize);

                    return Some((o, unsafe { core::slice::from_raw_parts(ptr, len as usize) }));
                }
            }
        }

        None
    }
}

pub fn send_discovery() {
    let total_len = core::mem::size_of::<DhcpHeader>();

    let mut packet = crate::kernel::net::udp::create_packet(
        68,
        67,
        total_len,
        Ip::limited_broadcast(), // 255.255.255.255
    );

    let header = unsafe { packet.addr.read_mut::<DhcpHeader>() };

    header.init();
    header.set_op(DhcpType::BootRequest);
    header.set_flags_broadcast(true);
    header.set_client_ip(Ip::empty());
    header.set_your_ip(Ip::empty());
    header.set_server_ip(Ip::empty());
    header.set_gateway_ip(Ip::empty());

    header
        .options_builder()
        .set_magic_cookie()
        .set_message_type(DhcpOptMsgType::DhcpDiscover)
        .set_client_identifier()
        .set_host_name("cykusz-os")
        .set_parameter_request_list()
        .finish();

    crate::kernel::net::udp::send_packet(packet);
}

fn send_request(requested_ip: Ip) {
    println!("Requesting ip {:?} from router", requested_ip);

    let total_len = core::mem::size_of::<DhcpHeader>();

    let mut packet = crate::kernel::net::udp::create_packet(
        68,
        67,
        total_len,
        Ip::limited_broadcast(), // 255.255.255.255
    );

    let header = unsafe { packet.addr.read_mut::<DhcpHeader>() };

    header.init();
    header.set_op(DhcpType::BootRequest);
    header.set_flags_broadcast(true);
    header.set_client_ip(Ip::empty());
    header.set_your_ip(Ip::empty());
    header.set_server_ip(Ip::empty());
    header.set_gateway_ip(Ip::empty());

    header
        .options_builder()
        .set_magic_cookie()
        .set_message_type(DhcpOptMsgType::DhcpRequest)
        .set_client_identifier()
        .set_host_name("cykusz-os")
        .set_requested_ip(requested_ip)
        .set_parameter_request_list()
        .finish();

    crate::kernel::net::udp::send_packet(packet);
}

pub fn process_packet(packet: Packet) {
    let header = unsafe { packet.addr.read_ref::<DhcpHeader>() };

    if let Some(mtype) = header.opt_message_type() {
        match mtype {
            DhcpOptMsgType::DhcpOffer => send_request(header.your_ip()),
            DhcpOptMsgType::DhcpAck => {
                println!("DHCP approved our request");
            }
            _ => {
                println!("Unexpected DHCP msg type: {:?}", mtype);
            }
        }
    }
}
