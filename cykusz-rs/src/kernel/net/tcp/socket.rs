use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::tcp::{Tcp, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader};
use crate::kernel::sync::Spin;

enum State {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    Closing,
}

impl Default for State {
    fn default() -> Self {
        State::Closed
    }
}

#[derive(Default)]
struct SocketData {
    their_seq: u32,
    our_seq: u32,
    src_port: u16,
    dst_port: u16,
    target: Ip4,
    state: State,
}

impl SocketData {
    pub fn new(port: u16) -> SocketData {
        SocketData {
            src_port: port,
            ..Default::default()
        }
    }

    pub fn src_port(&self) -> u16 {
        self.src_port
    }

    pub fn set_src_port(&mut self, val: u16) {
        self.src_port = val;
    }

    pub fn dst_port(&self) -> u16 {
        self.dst_port
    }

    pub fn set_dst_port(&mut self, val: u16) {
        self.dst_port = val;
    }

    pub fn target(&self) -> Ip4 {
        self.target
    }

    pub fn set_target(&mut self, val: Ip4) {
        self.target = val;
    }

    fn make_packet(&self, len: usize) -> Packet<Tcp> {
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, len, self.target);

        let out_hdr = out_packet.header_mut();

        out_hdr.set_seq_nr(self.our_seq);
        out_hdr.set_urgent_ptr(0);
        out_hdr.set_window(4096);

        out_packet
    }

    fn make_ack_packet(&self, len: usize) -> Packet<Tcp> {
        let mut packet = self.make_packet(len);

        let hdr = packet.header_mut();

        hdr.set_ack_nr(self.their_seq);

        packet
    }

    fn handle_listen(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, false) => {
                let ip = packet.downgrade();
                let iphdr = ip.header();

                self.dst_port = hdr.src_port();
                self.target = iphdr.src_ip;

                self.our_seq = 12345;
                self.their_seq = hdr.seq_nr().wrapping_add(1);

                let mut out = self.make_ack_packet(0);

                let outhdr = out.header_mut();

                outhdr.set_flag_syn(true);
                outhdr.set_flag_ack(true);

                println!("[ TCP ] Syn Received");

                self.state = State::SynReceived;

                Some(out)
            }
            _ => None,
        }
    }

    fn handle_syn_received(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_ack(), hdr.flag_rst()) {
            (true, false) => {
                self.our_seq = hdr.ack_nr();
                self.their_seq = hdr.seq_nr();

                println!("[ TCP ] Connection established");
                self.state = State::Established;

                None
            }
            (_, true) => {
                println!("[ TCP ] RST Received, Listening");
                self.state = State::Listen;
                None
            }
            _ => {
                println!("[ TCP ] Unknown state reached");
                None
            }
        }
    }

    fn handle_syn_sent(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, true) => {
                self.our_seq = hdr.ack_nr();
                self.their_seq = hdr.seq_nr().wrapping_add(1);

                let out = self.make_ack_packet(0);

                println!("[ TCP ] Connection Established");
                self.state = State::Established;

                Some(out)
            }
            _ => None,
        }
    }

    fn handle_established(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        if hdr.flag_ack() {
            self.our_seq = hdr.ack_nr();
        }

        if hdr.flag_fin() {
            self.their_seq = hdr.seq_nr().wrapping_add(1);

            let mut out = self.make_ack_packet(0);

            let outhdr = out.header_mut();

            outhdr.set_flag_fin(true);

            self.state = State::LastAck;

            Some(out)
        } else {
            None
        }
    }

    fn handle_finwait1(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_fin(), hdr.flag_ack()) {
            (false, true) => {
                self.our_seq = hdr.ack_nr();
                self.state = State::FinWait2;
                None
            }
            (true, true) => {
                self.our_seq = hdr.ack_nr();
                self.their_seq = hdr.seq_nr().wrapping_add(1);

                let out = self.make_ack_packet(0);

                self.state = State::Closed;
                crate::kernel::net::tcp::release_handler(self.src_port as u32);

                Some(out)
            }
            _ => {
                println!("[ TCP ] Unexpected FinWait1 packet");
                None
            }
        }
    }

    fn handle_finwait2(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_fin(), hdr.flag_ack()) {
            (true, false) => {
                self.their_seq = hdr.seq_nr().wrapping_add(1);

                let out = self.make_ack_packet(0);

                self.state = State::Closed;
                crate::kernel::net::tcp::release_handler(self.src_port as u32);

                Some(out)
            }
            _ => {
                println!("[ TCP ] Unexpected FinWait1 packet");
                None
            }
        }
    }

    fn handle_lastack(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        if hdr.flag_ack() {
            self.state = State::Closed;

            crate::kernel::net::tcp::release_handler(self.src_port as u32);
        }

        None
    }

    fn process(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        return match self.state {
            State::Listen => self.handle_listen(packet),
            State::SynReceived => self.handle_syn_received(packet),
            State::SynSent => self.handle_syn_sent(packet),
            State::Established => self.handle_established(packet),
            State::FinWait1 => self.handle_finwait1(packet),
            State::FinWait2 => self.handle_finwait2(packet),
            State::LastAck => self.handle_lastack(packet),
            _ => {
                println!("State not yet supported");
                None
            }
        };
    }

    fn close(&mut self) {
        println!("[ TCP ] Closing");
        let mut out_packet = self.make_ack_packet(0);

        let hdr = out_packet.header_mut();

        hdr.set_flag_fin(true);

        self.state = State::FinWait1;

        crate::kernel::net::tcp::send_packet(out_packet);
    }

    pub fn connect(&mut self) {
        self.our_seq = 12345;

        let mut packet = self.make_packet(0);

        let hdr = packet.header_mut();

        hdr.set_flag_syn(true);

        println!("[ TCP ] Connection Syn Sent");
        self.state = State::SynSent;

        crate::kernel::net::tcp::send_packet(packet);
    }

    pub fn listen(&mut self) {
        println!("[ TCP ] Listening");
        self.state = State::Listen;
    }
}

pub struct Socket {
    data: Spin<SocketData>,
}

impl Socket {
    pub fn new(port: u32) -> Socket {
        use core::convert::TryInto;
        Socket {
            data: Spin::new(SocketData::new(
                port.try_into().expect("Invalid port number"),
            )),
        }
    }

    pub fn connect(&self) {
        self.data.lock().connect();
    }

    pub fn listen(&self) {
        self.data.lock().listen()
    }

    pub fn src_port(&self) -> u16 {
        self.data.lock().src_port()
    }

    pub fn set_src_port(&self, val: u16) {
        self.data.lock().set_src_port(val);
    }

    pub fn dst_port(&self) -> u16 {
        self.data.lock().dst_port()
    }

    pub fn set_dst_port(&self, val: u16) {
        self.data.lock().set_dst_port(val);
    }

    pub fn target(&self) -> Ip4 {
        self.data.lock().target()
    }

    pub fn set_target(&self, val: Ip4) {
        self.data.lock().set_target(val);
    }
}

impl INode for Socket {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Ok(0)
    }

    fn poll_listen(&self, _listen: bool) -> Result<bool> {
        Ok(false)
    }

    fn poll_unlisten(&self) -> Result<()> {
        Ok(())
    }

    fn close(&self) {
        self.data.lock().close();
    }
}
impl TcpService for Socket {
    fn process_packet(&self, packet: Packet<Tcp>) {
        if let Some(packet) = self.data.lock().process(packet) {
            crate::kernel::net::tcp::send_packet(packet);
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);
    }
}

pub fn bind(port: u32) -> Option<Arc<dyn INode>> {
    let socket = Arc::new(Socket::new(port));

    if crate::kernel::net::tcp::register_handler(port, socket.clone()) {
        socket.listen();

        Some(socket)
    } else {
        None
    }
}

pub fn connect(host: Ip4, port: u32) -> Option<Arc<dyn INode>> {
    let socket = Arc::new(Socket::new(0));

    if let Some(p) = crate::kernel::net::tcp::register_ephemeral_handler(socket.clone()) {
        socket.set_src_port(p as u16);
        socket.set_dst_port(port as u16);
        socket.set_target(host);

        socket.connect();

        Some(socket)
    } else {
        None
    }
}
