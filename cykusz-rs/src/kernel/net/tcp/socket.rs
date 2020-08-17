use alloc::sync::{Arc, Weak};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::net::ip::{Ip4, IpHeader};
use crate::kernel::net::tcp::{Tcp, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader, PacketTrait};
use crate::kernel::sync::Spin;
use crate::kernel::syscall::sys::PollTable;
use crate::kernel::timer::{create_timer, Timer, TimerObject};
use crate::kernel::utils::buffer::BufferQueue;
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

#[derive(Default, Debug)]
struct TransmissionCtl {
    snd_nxt: u32,
    snd_una: u32,
    rcv_nxt: u32,
}

#[derive(Default)]
struct SocketData {
    src_port: u16,
    dst_port: u16,
    target: Ip4,
    state: State,
    timer: Option<Arc<Timer>>,
    ctl: TransmissionCtl,
    buffer: BufferQueue,
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

    fn timer(&self) -> &Arc<Timer> {
        self.timer.as_ref().unwrap()
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

    fn make_packet(&mut self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, len, self.target);

        let out_hdr = out_packet.header_mut();

        out_hdr.set_seq_nr(self.ctl.snd_nxt);
        out_hdr.set_urgent_ptr(0);
        out_hdr.set_window(4096);

        out_hdr.set_flags(flags);

        let una = if len > 0 {
            len as u32
        } else if (flags & (TcpFlags::FIN | TcpFlags::SYN)).bits > 0 {
            1
        } else {
            0
        };

        self.ctl.snd_una += una;

        out_packet
    }

    fn make_ack_packet(&mut self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut packet = self.make_packet(len, flags);

        let hdr = packet.header_mut();

        hdr.set_ack_nr(self.ctl.rcv_nxt);

        packet
    }

    fn setup_connection(&mut self, packet: Packet<Tcp>) {
        let ip = packet.downgrade();
        let hdr = packet.header();
        let iphdr = ip.header();

        self.dst_port = hdr.src_port();
        self.target = iphdr.src_ip;

        self.ctl.snd_nxt = 12345;
        self.ctl.snd_una = 12345;
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
            use core::mem::size_of;

            let ip = packet.downgrade();

            let data_len = ip.header().len.value() as usize
                - size_of::<IpHeader>() as usize
                - hdr.header_len() as usize;

            let data = &packet.data()[..data_len];

            if hdr.flag_ack() {
                self.ctl.snd_nxt = hdr.ack_nr();
            }

            if !data.is_empty() {
                self.ctl.rcv_nxt = hdr.seq_nr().wrapping_add(data_len as u32);

                let out = self.make_ack_packet(0, TcpFlags::empty());

                self.buffer.append_data(data);

                Some(out)
            } else {
                None
            }
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
            self.ctl.snd_nxt = hdr.ack_nr();
        }

        self.ctl.rcv_nxt = if hdr.flag_syn() || hdr.flag_fin() {
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

        self.timer().terminate();

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
        self.ctl.snd_nxt = 12345;
        self.ctl.snd_una = 12345;

        let packet = self.make_packet(0, TcpFlags::SYN);

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
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        let data = self.data.lock();

        Ok(data.buffer.read_data(buf))
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        let mut data = self.data.lock();

        let mut packet = data.make_ack_packet(buf.len(), TcpFlags::PSH);

        packet.data_mut().copy_from_slice(buf);

        crate::kernel::net::tcp::send_packet(packet);

        Ok(buf.len())
    }

    fn poll(&self, listen: Option<&mut PollTable>) -> Result<bool> {
        let data = self.data.lock();

        if let Some(pt) = listen {
            pt.listen(&data.buffer.wait_queue());
        }

        if data.state == State::Closed && !data.buffer.has_data() {
            Err(FsError::NotSupported)
        } else {
            Ok(data.buffer.has_data())
        }
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
            data.buffer.wait_queue().notify_all();
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        println!("[ TCP ] Socket Removed");
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
