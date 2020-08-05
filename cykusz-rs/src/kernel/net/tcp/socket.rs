use alloc::sync::{Arc, Weak};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::tcp::{Tcp, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader};
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::timer::{create_timer, Timer, TimerObject};
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(PartialEq)]
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

bitflags! {
    pub struct TcpFlags: u16 {
        const FIN   = 1 << 0;
        const SYN   = 1 << 1;
        const RST   = 1 << 2;
        const PSH   = 1 << 3;
        const ACK   = 1 << 4;
        const URG   = 1 << 5;
    }
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
    timer: Option<Arc<Timer>>,
}

impl SocketData {
    pub fn new(port: u16) -> SocketData {
        SocketData {
            src_port: port,
            ..Default::default()
        }
    }

    fn init_timers(&mut self, obj: Arc<dyn TimerObject>) {
        self.timer = Some(create_timer(obj, 1000));
    }

    fn timeout(&mut self) {
        println!("[ TCP ] Timeout");
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

    fn make_packet(&self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, len, self.target);

        let out_hdr = out_packet.header_mut();

        out_hdr.set_seq_nr(self.our_seq);
        out_hdr.set_urgent_ptr(0);
        out_hdr.set_window(4096);

        out_hdr.set_flags(flags);

        out_packet
    }

    fn make_ack_packet(&self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut packet = self.make_packet(len, flags);

        let hdr = packet.header_mut();

        hdr.set_ack_nr(self.their_seq);

        packet
    }

    fn setup_connection(&mut self, packet: Packet<Tcp>) {
        let ip = packet.downgrade();
        let hdr = packet.header();
        let iphdr = ip.header();

        self.dst_port = hdr.src_port();
        self.target = iphdr.src_ip;

        self.our_seq = 12345;
    }

    fn handle_listen(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, false) => {
                self.setup_connection(packet);

                let out = self.make_ack_packet(0, TcpFlags::SYN);

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
                let out = self.make_ack_packet(0, TcpFlags::empty());

                println!("[ TCP ] Connection Established");

                self.state = State::Established;

                Some(out)
            }
            _ => None,
        }
    }

    fn handle_established(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        if hdr.flag_fin() {
            let out = self.make_ack_packet(0, TcpFlags::FIN);

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
                self.state = State::FinWait2;
                None
            }
            (true, true) => {
                let out = self.make_ack_packet(0, TcpFlags::empty());

                self.finalize();

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
            (true, true) => {
                let out = self.make_ack_packet(0, TcpFlags::empty());

                self.finalize();

                Some(out)
            }
            _ => {
                println!("[ TCP ] Unexpected FinWait2 packet");
                None
            }
        }
    }

    fn handle_lastack(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        if hdr.flag_ack() {
            self.finalize();
        }

        None
    }

    fn process(&mut self, packet: Packet<Tcp>) -> Option<Packet<Tcp>> {
        let hdr = packet.header();

        if hdr.flag_rst() {
            self.finalize();
            return None;
        }

        if hdr.flag_ack() {
            self.our_seq = hdr.ack_nr();
        }

        self.their_seq = if hdr.flag_syn() || hdr.flag_fin() {
            hdr.seq_nr().wrapping_add(1)
        } else {
            hdr.seq_nr()
        };

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

    fn finalize(&mut self) {
        self.state = State::Closed;

        println!("[ TCP ] Connection closed by RST");

        self.timer.as_ref().unwrap().set_terminate();
        self.timer = None;

        crate::kernel::net::tcp::release_handler(self.src_port as u32);
    }

    fn close(&mut self) {
        if self.state != State::Closed {
            println!("[ TCP ] Closing");
            let out_packet = self.make_ack_packet(0, TcpFlags::FIN);

            self.state = State::FinWait1;

            crate::kernel::net::tcp::send_packet(out_packet);
        }
    }

    pub fn connect(&mut self) {
        self.our_seq = 12345;

        let packet = self.make_packet(0, TcpFlags::SYN);

        println!("[ TCP ] Connection Syn Sent");

        self.state = State::SynSent;

        crate::kernel::net::tcp::send_packet(packet);

        //self.timer.as_ref().unwrap().resume();
    }

    pub fn listen(&mut self) {
        println!("[ TCP ] Listening");
        self.state = State::Listen;
    }
}

pub struct Socket {
    data: Spin<SocketData>,
    wait_queue: WaitQueue,

    self_ref: Weak<Socket>,
}

impl TimerObject for Socket {
    fn call(&self) {
        self.data.lock().timeout();
    }
}

impl Socket {
    pub fn new(port: u32) -> Arc<Socket> {
        use core::convert::TryInto;
        let sock = Socket {
            data: Spin::new(SocketData::new(
                port.try_into().expect("Invalid port number"),
            )),
            wait_queue: WaitQueue::new(),
            self_ref: Weak::default(),
        }
        .wrap();

        sock.data.lock().init_timers(sock.clone());

        sock
    }

    fn wrap(self) -> Arc<Socket> {
        let fs = Arc::new(self);
        let weak = Arc::downgrade(&fs);
        let ptr = Arc::into_raw(fs) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
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

    fn poll_listen(&self, listen: bool) -> Result<bool> {
        if listen {
            self.wait_queue.add_task(current_task());
        }

        if self.data.lock().state == State::Closed {
            Err(FsError::NotSupported)
        } else {
            Ok(false)
        }
    }

    fn poll_unlisten(&self) -> Result<()> {
        self.wait_queue.remove_task(current_task());

        Ok(())
    }

    fn close(&self) {
        self.data.lock().close();
    }
}
impl TcpService for Socket {
    fn process_packet(&self, packet: Packet<Tcp>) {
        let mut data = self.data.lock();
        if let Some(packet) = data.process(packet) {
            crate::kernel::net::tcp::send_packet(packet);
        }

        if data.state == State::Closed {
            self.wait_queue.notify_one();
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);
    }
}

pub fn bind(port: u32) -> Option<Arc<dyn INode>> {
    let socket = Socket::new(port);

    if crate::kernel::net::tcp::register_handler(port, socket.clone()) {
        socket.listen();

        Some(socket)
    } else {
        None
    }
}

pub fn connect(host: Ip4, port: u32) -> Option<Arc<dyn INode>> {
    let socket = Socket::new(0);

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