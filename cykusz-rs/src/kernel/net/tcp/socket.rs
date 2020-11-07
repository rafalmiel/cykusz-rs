use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::net::ip::{Ip4, IpHeader};
use crate::kernel::net::tcp::{Tcp, TcpHeader, TcpService};
use crate::kernel::net::{Packet, PacketDownHierarchy, PacketHeader, PacketTrait};
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::syscall::sys::PollTable;
use crate::kernel::timer::{create_timer, current_ns, Timer, TimerObject};
use crate::kernel::utils::buffer::BufferQueue;
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(PartialEq, Debug)]
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
    snd_wnd: u32,
    iss: u32,
    rcv_nxt: u32,
    rcv_wnd: u32,
    irs: u32,
}

impl TransmissionCtl {
    fn available_window(&self) -> usize {
        let outstanding = self.snd_nxt.wrapping_sub(self.snd_una);

        if outstanding > self.snd_wnd {
            0
        } else {
            self.snd_wnd as usize - outstanding as usize
        }
    }
}

#[derive(Default)]
struct ConnTimer {
    timeout: u64,
    timeout_count: usize,
    conn_timer: Option<Arc<Timer>>,
}

impl ConnTimer {
    fn timer(&self) -> &Arc<Timer> {
        self.conn_timer.as_ref().unwrap()
    }

    fn reset(&mut self) {
        self.timeout = 1000;
        self.timeout_count = 0;
        self.timer().disable();
    }

    fn start_with_timeout(&self, timeout: u64) {
        self.timer().start_with_timeout(timeout);
    }

    fn conn_timeout_update(&mut self) -> bool {
        self.timeout_count += 1;

        if self.timeout_count <= 5 {
            self.timeout *= 2;

            true
        } else {
            self.reset();

            false
        }
    }

    fn conn_timeout_start(&self) {
        self.timer().start_with_timeout(self.timeout)
    }
}

#[derive(Default)]
struct SocketData {
    src_port: u16,
    dst_port: u16,
    target: Ip4,
    state: State,
    conn_timer: ConnTimer,
    rx_timer: Option<Arc<Timer>>,
    tx_timer: Option<Arc<Timer>>,
    dc_timer: Option<Arc<Timer>>,
    ctl: TransmissionCtl,
    in_buffer: BufferQueue,
    out: PacketQueue,
    send_queue: WaitQueue,
}

struct QueuedPacket {
    packet: Packet<Tcp>,
    ack: u32,
}

impl Drop for QueuedPacket {
    fn drop(&mut self) {
        self.packet.deallocate();
    }
}

struct PacketQueue {
    queue: Vec<QueuedPacket>,
}

impl Default for PacketQueue {
    fn default() -> PacketQueue {
        PacketQueue { queue: Vec::new() }
    }
}

struct TimerCallback {
    sock: Weak<Socket>,
    fun: fn(&Socket),
}

impl TimerObject for TimerCallback {
    fn call(&self) {
        if let Some(s) = self.sock.upgrade() {
            (self.fun)(&s)
        }
    }
}

impl TimerCallback {
    fn new(sock: Arc<Socket>, cb: fn(&Socket)) -> Arc<TimerCallback> {
        Arc::new(TimerCallback {
            sock: Arc::downgrade(&sock),
            fun: cb,
        })
    }
}

impl SocketData {
    pub fn new(port: u16) -> SocketData {
        SocketData {
            src_port: port,
            in_buffer: BufferQueue::new(4096 * 18),
            ..Default::default()
        }
    }

    fn init_timers(&mut self, obj: Arc<Socket>) {
        self.conn_timer = ConnTimer {
            timeout_count: 0,
            timeout: 1000,
            conn_timer: Some(create_timer(TimerCallback::new(
                obj.clone(),
                Socket::conn_timeout,
            ))),
        };
        self.rx_timer = Some(create_timer(TimerCallback::new(
            obj.clone(),
            Socket::rx_timeout,
        )));
        self.tx_timer = Some(create_timer(TimerCallback::new(
            obj.clone(),
            Socket::tx_timeout,
        )));
        self.dc_timer = Some(create_timer(TimerCallback::new(obj, Socket::dc_timeout)));
    }

    fn stop_timers(&mut self) {
        self.conn_timer().reset();
        self.rx_timer().disable();
        self.tx_timer().disable();
        self.dc_timer().disable();
    }

    fn conn_timer(&mut self) -> &mut ConnTimer {
        &mut self.conn_timer
    }

    fn rx_timer(&self) -> &Arc<Timer> {
        self.rx_timer.as_ref().unwrap()
    }

    fn tx_timer(&self) -> &Arc<Timer> {
        self.tx_timer.as_ref().unwrap()
    }

    fn dc_timer(&self) -> &Arc<Timer> {
        self.dc_timer.as_ref().unwrap()
    }

    fn conn_timeout(&mut self) {
        println!("[ TCP ] SyncSent Timeout");

        if self.conn_timer().conn_timeout_update() {
            self.send_sync();

            self.conn_timer().conn_timeout_start();
        } else {
            self.finalize();
        }
    }

    fn rx_timeout(&mut self) {
        self.send_ack();
    }

    fn tx_timeout(&mut self) {
        let out = if let Some(p) = self.out.queue.get(0) {
            Some(p.packet)
        } else {
            None
        };

        if let Some(mut p) = out {
            let hdr = p.header_mut();

            hdr.set_ack_nr(self.ctl.rcv_nxt);

            self.resend_packet(p);

            self.tx_timer().start_with_timeout(1000);
        } else {
            self.tx_timer().disable();
        }
    }

    fn dc_timeout(&mut self) {
        println!("[ TCP ] LastAck Timeout");
        self.finalize();
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

    fn resend_packet(&self, packet: Packet<Tcp>) {
        crate::kernel::net::tcp::send_packet(packet);
    }

    fn send_packet(&mut self, packet: Packet<Tcp>, queue: bool) {
        crate::kernel::net::tcp::send_packet(packet);

        self.ctl.snd_nxt = self.ctl.snd_nxt.wrapping_add(packet.ack_len());

        //println!("[ TCP ] Send window: {} >= {}", self.ctl.snd_wnd, self.ctl.snd_nxt.wrapping_sub(self.ctl.snd_una));

        if queue {
            let queued = QueuedPacket {
                packet,
                ack: self.ctl.snd_nxt,
            };

            let start_timer = self.out.queue.is_empty();

            self.out.queue.push(queued);

            if start_timer {
                self.tx_timer().start_with_timeout(1000);
            }
        }
    }

    fn send_sync(&mut self) {
        let packet = self.make_packet(0, TcpFlags::SYN);

        println!("[ TCP ] Connection Syn Sent");

        self.state = State::SynSent;

        self.send_packet(packet, false);
    }

    fn send_ack(&mut self) {
        let out = self.make_ack_packet(0, TcpFlags::empty());

        self.send_packet(out, false);
    }

    fn send_ack_flags(&mut self, flags: TcpFlags) {
        let out = self.make_ack_packet(0, flags);

        self.send_packet(out, false);
    }

    fn process_ack(&mut self, header: &TcpHeader) {
        if self.ctl.snd_una != header.ack_nr() {
            //println!("[ TCP ] Update snd_una {} -> {}", self.ctl.snd_una, header.ack_nr());
            self.ctl.snd_una = header.ack_nr();

            self.send_queue.notify_all();

            let a = self
                .out
                .queue
                .iter()
                .enumerate()
                .find(|p| p.1.ack >= header.ack_nr());

            if let Some((idx, _)) = a {
                for i in 0..idx {
                    self.out.queue.remove(0);
                }

                if self.out.queue.get(0).unwrap().ack == header.ack_nr() {
                    self.out.queue.remove(0);
                }

                //println!("[ TCP ] Removed {} packets from the queue", idx);

                if !self.out.queue.is_empty() {
                    self.tx_timer().start_with_timeout(1000);
                } else {
                    self.tx_timer().disable();
                }
            }
        }

        //println!("[ TCP ] Resend queue len {}", self.out.queue.len());

        //if !self.out.queue.is_empty() {
        //    for e in self.out.queue.iter() {
        //        print!("{} ", e.ack);
        //    }
        //    println!(".");
        //}
    }

    fn make_packet(&mut self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, len, self.target);

        let out_hdr = out_packet.header_mut();

        out_hdr.set_seq_nr(self.ctl.snd_nxt);
        out_hdr.set_urgent_ptr(0);
        out_hdr.set_window(
            core::cmp::min(u16::max_value() as usize, self.in_buffer.available_size()) as u16,
        );

        out_hdr.set_flags(flags);

        out_packet
    }

    fn make_ack_packet(&mut self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut packet = self.make_packet(len, flags);

        let hdr = packet.header_mut();

        hdr.set_ack_nr(self.ctl.rcv_nxt);

        packet
    }

    fn init_seq(&mut self) {
        let v = current_ns() as u32;

        self.ctl.snd_nxt = v;
        self.ctl.snd_una = v;
    }

    fn setup_connection(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();
        let ip = packet.downgrade();
        let ip_header: &IpHeader = ip.header();

        self.dst_port = hdr.src_port();
        self.target = ip_header.src_ip;

        self.init_seq();
    }

    fn update_rcv_next(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();
        self.ctl.rcv_nxt = hdr.seq_nr().wrapping_add(packet.ack_len());
    }

    fn handle_listen(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, false) => {
                self.ctl.iss = hdr.seq_nr();
                self.ctl.irs = hdr.seq_nr();

                self.setup_connection(packet);

                self.update_rcv_next(packet);

                self.send_ack_flags(TcpFlags::SYN);

                println!("[ TCP ] Syn Received");

                self.state = State::SynReceived;
            }
            _ => {}
        }
    }

    fn handle_syn_received(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_ack(), hdr.flag_rst()) {
            (true, false) => {
                self.conn_timer().reset();
                println!("[ TCP ] Connection established");
                self.state = State::Established;
            }
            (_, true) => {
                println!("[ TCP ] RST Received, Listening");
                self.state = State::Listen;
            }
            _ => {
                println!("[ TCP ] Unknown state reached");
            }
        }
    }

    fn handle_syn_sent(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, true) => {
                self.ctl.iss = hdr.seq_nr();
                self.ctl.irs = hdr.seq_nr();

                self.update_rcv_next(packet);

                println!("[ TCP ] Connection Established");

                self.conn_timer().reset();

                self.send_ack();

                self.state = State::Established;
            }
            _ => {}
        }
    }

    fn handle_established_close_wait(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        let data = packet.data();

        if hdr.flag_fin() {
            self.state = State::CloseWait;
        }

        if !data.is_empty() || hdr.flag_fin() {
            self.in_buffer.append_data(data);

            self.send_ack();

            if hdr.flag_fin() && data.is_empty() {
                self.in_buffer.wait_queue().notify_all();
            }
        }
    }

    fn handle_established(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        let data = packet.data();

        if !data.is_empty() || hdr.flag_fin() {
            if hdr.seq_nr() == self.ctl.rcv_nxt {
                if self.in_buffer.append_data(data) == data.len() {
                    //println!("[ TCP ] Stored {} bytes", data.len());
                    self.update_rcv_next(packet);

                    self.ctl.rcv_wnd += packet.ack_len();

                    if self.ctl.rcv_wnd >= 4096 * 4 {
                        self.send_ack();

                        self.ctl.rcv_wnd = 0;
                        self.rx_timer().disable();
                    } else {
                        self.rx_timer().start_with_timeout(200);
                    }
                } else {
                    println!("[ TCP ] Failed to store data");
                }
            }

            if hdr.flag_fin() {
                self.state = State::LastAck;
                self.send_ack_flags(TcpFlags::FIN);
                self.dc_timer().start_with_timeout(1000);
            }
        }
    }

    fn handle_finwait1(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_fin(), hdr.flag_ack()) {
            (false, true) => {
                self.state = State::FinWait2;
            }
            (true, true) => {
                self.update_rcv_next(packet);

                self.send_ack();

                self.finalize();
            }
            (true, false) => {
                self.update_rcv_next(packet);

                self.send_ack();

                self.state = State::Closing;

                self.dc_timer().start_with_timeout(500);
            }
            _ => {
                println!("[ TCP ] Unexpected FinWait1 packet");
                self.send_ack_flags(TcpFlags::RST);

                self.finalize();
            }
        }
    }

    fn handle_finwait2(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_fin(), hdr.flag_ack()) {
            (true, true) => {
                self.update_rcv_next(packet);

                self.send_ack();

                self.finalize();
            }
            _ => {
                println!("[ TCP ] Unexpected FinWait2 packet");
                self.send_ack_flags(TcpFlags::RST);

                self.finalize();
            }
        }
    }

    fn handle_closing(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        if hdr.flag_ack() {
            self.finalize();
        }
    }

    fn handle_lastack(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        if hdr.flag_ack() {
            self.finalize();
        }
    }

    fn process(&mut self, packet: Packet<Tcp>) {
        //print!(".");
        let hdr = packet.header();

        self.ctl.snd_wnd = hdr.window() as u32;

        //if packet.data().len() == 0 && hdr.flag_ack() && hdr.ack_nr() == self.ctl.snd_una {
        //    println!("[ TCP ] Dup ack");
        //    self.tx_timeout();
        //}

        if hdr.flag_ack() {
            self.process_ack(hdr);
        }

        if self.state != State::Listen && self.state != State::SynSent {
            if self.ctl.rcv_nxt != hdr.seq_nr() {
                println!(
                    "[ TCP ] Out of Order Packet Received {}, {} vs {}",
                    (hdr.seq_nr() - self.ctl.irs),
                    self.ctl.rcv_nxt,
                    hdr.seq_nr()
                );

                println!("[ TCP ] Available buffer: {}", self.in_buffer.available_size());

                //self.send_ack();
                //self.send_ack();
                //self.send_ack_flags(TcpFlags::RST);
                //self.finalize();
                //return;
            }
        }

        match self.state {
            State::Listen => self.handle_listen(packet),
            State::SynReceived => self.handle_syn_received(packet),
            State::SynSent => self.handle_syn_sent(packet),
            State::Established => self.handle_established(packet),
            State::FinWait1 => self.handle_finwait1(packet),
            State::FinWait2 => self.handle_finwait2(packet),
            State::LastAck => self.handle_lastack(packet),
            State::Closing => self.handle_closing(packet),
            _ => {
                println!("State not yet supported");
            }
        };

        if hdr.flag_rst() && self.state != State::Closed && self.state != State::Listen {
            self.finalize();
        }
    }

    fn finalize(&mut self) {
        assert_ne!(
            self.state,
            State::Closed,
            "[ TCP ] Finalize called on closed socket"
        );

        self.state = State::Closed;

        println!("[ TCP ] Connection closed");

        self.stop_timers();

        crate::kernel::net::tcp::release_handler(self.src_port as u32);

        self.in_buffer.wait_queue().notify_all();
    }

    fn close(&mut self) {
        println!("[ TCP ] Closing");
        match self.state {
            State::Established | State::SynReceived => {
                self.send_ack_flags(TcpFlags::FIN);

                self.state = State::FinWait1;
            }
            State::CloseWait => {
                println!("[ TCP ] Close wait sent fin");
                self.send_ack_flags(TcpFlags::FIN);

                self.state = State::LastAck;

                self.dc_timer().start_with_timeout(500);
            }
            _ if self.state != State::Closed => {
                self.finalize();
            }
            _ => {}
        }
    }

    pub fn connect(&mut self) {
        self.init_seq();

        self.send_sync();

        self.conn_timer().reset();
        self.conn_timer().conn_timeout_start();
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

    fn conn_timeout(&self) {
        self.data.lock().conn_timeout();
    }

    fn rx_timeout(&self) {
        self.data.lock().rx_timeout();
    }

    fn tx_timeout(&self) {
        self.data.lock().tx_timeout();
    }

    fn dc_timeout(&self) {
        self.data.lock().dc_timeout();
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
        let mut data = self.data.lock();

        let wnd_update = data.in_buffer.available_size() == 0;

        let r = data.in_buffer.read_data(buf);

        if wnd_update {
            data.send_ack();
        }

        //println!("[ TCP ] Read {} bytes", r);

        Ok(r)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        let mut data = self.data.lock();

        if data.state == State::Established {
            let task = current_task();

            data.send_queue.add_task(task.clone());

            while data.ctl.available_window() < buf.len() && data.state == State::Established {
                //println!("[ TCP ] Awaiting available window {} {}", data.ctl.available_window(), buf.len());
                WaitQueue::wait_lock(data);

                data = self.data.lock();
            }

            data.send_queue.remove_task(task);

            if data.state != State::Established {
                return Err(FsError::NotSupported);
            }

            let mut packet = data.make_ack_packet(buf.len(), TcpFlags::empty());

            packet.data_mut().copy_from_slice(buf);

            data.send_packet(packet, true);

            data.ctl.rcv_wnd = 0;
            data.rx_timer().disable();

            Ok(buf.len())
        } else {
            // Buffer data for when connection is ready?
            Err(FsError::NotSupported)
        }
    }

    fn poll(&self, listen: Option<&mut PollTable>) -> Result<bool> {
        let data = self.data.lock();

        if let Some(pt) = listen {
            pt.listen(&data.in_buffer.wait_queue());
        }

        if (data.state == State::Closed || data.state == State::CloseWait)
            && !data.in_buffer.has_data()
        {
            Err(FsError::NotSupported)
        } else {
            Ok(data.in_buffer.has_data())
        }
    }

    fn close(&self) {
        self.data.lock().close();
    }
}

impl TcpService for Socket {
    fn process_packet(&self, packet: Packet<Tcp>) {
        let mut data = self.data.lock();

        data.process(packet);

        if data.state == State::Closed {
            data.in_buffer.wait_queue().notify_all();
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);

        self.data.lock().finalize();
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
