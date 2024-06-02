use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use syscall_defs::net::{MsgFlags, MsgHdr, SockAddrIn, SockAddrPtr, SockOption};
use syscall_defs::poll::PollEventFlags;
use syscall_defs::stat::Stat;
use syscall_defs::{OpenFlags, SyscallError, SyscallResult};

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::net::ip::{Ip, Ip4, IpHeader};
use crate::kernel::net::socket::{NetSocketService, SocketService};
use crate::kernel::net::tcp::{Tcp, TcpHeader};
use crate::kernel::net::{
    default_driver, Packet, PacketDownHierarchy, PacketHeader, PacketTrait, PacketUpHierarchy,
};
use crate::kernel::sched::current_task;
use crate::kernel::sync::{LockApi, Mutex, Spin};
use crate::kernel::timer::{create_timer, current_ns, Timer, TimerCallback};
use crate::kernel::utils::buffer::{Buffer, BufferQueue};
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

#[derive(PartialEq, Debug, Copy, Clone)]
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

impl core::default::Default for State {
    fn default() -> Self {
        State::Closed
    }
}

#[derive(Default, Debug, Copy, Clone)]
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
    proxy_buffer: BufferQueue,
    snd_buffer: Buffer,
    socket: Weak<Socket>,
}

impl SocketData {
    pub fn new(port: u16, socket: &Weak<Socket>) -> SocketData {
        SocketData {
            src_port: port,
            proxy_buffer: BufferQueue::new(4096 * 8, true, true),
            snd_buffer: Buffer::new(4096 * 18),
            socket: socket.clone(),
            ..Default::default()
        }
    }

    pub fn new_unbound(socket: &Weak<Socket>) -> SocketData {
        SocketData {
            proxy_buffer: BufferQueue::new_empty(true, true),
            snd_buffer: Buffer::new_empty(),
            socket: socket.clone(),
            ..Default::default()
        }
    }

    pub fn new_connected(
        from: &SocketData,
        packet: Packet<Tcp>,
        socket: &Weak<Socket>,
    ) -> SocketData {
        let mut data = SocketData {
            proxy_buffer: BufferQueue::new_empty(true, true),
            snd_buffer: Buffer::new_empty(),
            ctl: from.ctl,
            socket: socket.clone(),
            ..Default::default()
        };

        data.init_connected(from.src_port(), packet);

        data
    }

    fn init(&mut self, listening: bool) {
        if !listening {
            self.proxy_buffer.init_size(4096 * 8);
            self.snd_buffer.init_size(4096 * 18);
        }
        self.init_timers();
    }

    fn socket(&self) -> Arc<Socket> {
        self.socket.upgrade().unwrap()
    }

    fn init_timers(&mut self) {
        let obj = self.socket();

        self.conn_timer = ConnTimer {
            timeout_count: 0,
            timeout: 1000,
            conn_timer: Some(create_timer(TimerCallback::new(
                Arc::downgrade(&obj),
                Socket::conn_timeout,
            ))),
        };
        self.rx_timer = Some(create_timer(TimerCallback::new(
            Arc::downgrade(&obj),
            Socket::rx_timeout,
        )));
        self.tx_timer = Some(create_timer(TimerCallback::new(
            Arc::downgrade(&obj),
            Socket::tx_timeout,
        )));
        self.dc_timer = Some(create_timer(TimerCallback::new(
            Arc::downgrade(&obj),
            Socket::dc_timeout,
        )));
    }

    fn stop_timers(&mut self) {
        self.conn_timer().reset();
        self.rx_timer().disable();
        self.tx_timer().disable();
        self.dc_timer().disable();
    }

    fn is_listening(&self) -> bool {
        self.state == State::Listen
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
        logln_disabled!("[ TCP ] SyncSent Timeout");

        if self.conn_timer().conn_timeout_update() {
            self.send_sync();

            self.conn_timer().conn_timeout_start();
        } else {
            self.finalize();
        }
    }

    fn rx_timeout(&mut self) {
        logln5!("rx timeout send ack");
        self.send_ack();
    }

    fn tx_timeout(&mut self) {
        if self.snd_buffer.has_data() {
            let mut data = [0u8; 1460];

            for o in (0..self.snd_buffer.size()).step_by(1460) {
                //let read = self.snd_buffer.read_data_transient(&mut data);
                let mut len = core::cmp::min(data.len(), self.snd_buffer.size() - o);

                let mut packet = self.make_ack_packet(len, TcpFlags::empty());

                let hdr = packet.header_mut();

                hdr.set_seq_nr(self.ctl.snd_una + o as u32);

                len = self
                    .snd_buffer
                    .read_data_transient_from(0, &mut data[..len]);

                packet.data_mut().copy_from_slice(&data[..len]);

                self.resend_packet(packet);

                break;
            }

            self.tx_timer().start_with_timeout(1000);
        } else {
            self.tx_timer().disable();
        }
    }

    fn dc_timeout(&mut self) {
        logln_disabled!("[ TCP ] LastAck Timeout");
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
        self.ctl.snd_nxt = self.ctl.snd_nxt.wrapping_add(packet.ack_len());

        //println!("[ TCP ] Send window: {} >= {}", self.ctl.snd_wnd, self.ctl.snd_nxt.wrapping_sub(self.ctl.snd_una));

        if queue {
            let start_timer = self.snd_buffer.size() == 0;

            self.snd_buffer.append_data(packet.data());

            if start_timer {
                self.tx_timer().start_with_timeout(1000);
            }
        }

        crate::kernel::net::tcp::send_packet(packet);
    }

    fn send_sync(&mut self) {
        let packet = self.make_packet(0, TcpFlags::SYN);

        logln_disabled!("[ TCP ] Connection Syn Sent");

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

    fn drain_proxy_buffer(&mut self) {
        if self.state != State::Established {
            return;
        }

        let mut window = self.ctl.available_window();

        let max = 1460usize;

        while self.proxy_buffer.has_data() && window > 0 {
            let cap = core::cmp::min(self.proxy_buffer.size(), core::cmp::min(max, window));

            let mut packet = self.make_ack_packet(cap, TcpFlags::empty());

            self.proxy_buffer
                .read_data(&mut packet.data_mut()[..cap])
                .expect("[ NET ] Unexpected signal in process_ack");

            //logln4!("send ack");
            self.send_packet(packet, true);

            window = self.ctl.available_window();
        }

        self.proxy_buffer.writers_queue().notify_one();
    }

    fn process_ack(&mut self, header: &TcpHeader) {
        if self.ctl.snd_una != header.ack_nr() {
            //TODO: Do a better check here (in case of wrap around)

            let bytes_acked = header.ack_nr().wrapping_sub(self.ctl.snd_una);

            self.snd_buffer.mark_as_read(bytes_acked as usize);

            self.ctl.snd_una = header.ack_nr();

            self.drain_proxy_buffer();
        }
    }

    fn make_packet(&mut self, len: usize, flags: TcpFlags) -> Packet<Tcp> {
        let mut out_packet =
            crate::kernel::net::tcp::create_packet(self.src_port, self.dst_port, len, self.target);

        let out_hdr = out_packet.header_mut();

        out_hdr.set_seq_nr(self.ctl.snd_nxt);
        out_hdr.set_urgent_ptr(0);
        out_hdr.set_window(core::cmp::min(
            u16::MAX as usize,
            self.socket().in_buffer().available_size(),
        ) as u16);

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
    }

    fn update_rcv_next(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();
        logln5!(
            "update rcv next {} -> {}",
            self.ctl.rcv_nxt,
            hdr.seq_nr().wrapping_add(packet.ack_len())
        );
        self.ctl.rcv_nxt = hdr.seq_nr().wrapping_add(packet.ack_len());
    }

    fn handle_listen(&mut self, _packet: Packet<Tcp>) {
        panic!("Unexpected listen on a non-listening socket");
    }

    fn handle_syn_received(&mut self, packet: Packet<Tcp>) {
        let hdr = packet.header();

        match (hdr.flag_ack(), hdr.flag_rst()) {
            (true, false) => {
                self.conn_timer().reset();
                logln_disabled!("[ TCP ] Connection established");
                self.state = State::Established;

                self.drain_proxy_buffer();
            }
            (_, true) => {
                logln_disabled!("[ TCP ] RST Received, Listening");
                self.finalize();
            }
            _ => {
                logln_disabled!("[ TCP ] Unknown state reached");
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

                self.state = State::Established;
                logln5!("[ TCP ] Connection Established");

                self.conn_timer().reset();

                self.send_ack();

                self.drain_proxy_buffer();
            }
            _ => {}
        }
    }

    fn handle_established_close_wait(&mut self, packet: Packet<Tcp>) {
        let socket = self.socket();
        let in_buffer = socket.in_buffer();

        let hdr = packet.header();

        let data = packet.data();

        if hdr.flag_fin() {
            self.state = State::CloseWait;
        }

        if !data.is_empty() || hdr.flag_fin() {
            in_buffer.try_append_data(data);

            self.send_ack();

            if hdr.flag_fin() && data.is_empty() {
                in_buffer.readers_queue().notify_all();
            }
        }
    }

    fn handle_established(&mut self, packet: Packet<Tcp>) {
        let socket = self.socket();
        let in_buffer = socket.in_buffer();

        let hdr = packet.header();

        let data = packet.data();

        if !data.is_empty() || hdr.flag_fin() {
            if hdr.seq_nr() == self.ctl.rcv_nxt {
                if in_buffer.try_append_data(data) == data.len() {
                    logln5!("[ TCP ] Stored {} bytes", data.len());
                    self.update_rcv_next(packet);

                    self.ctl.rcv_wnd += packet.ack_len();

                    if self.ctl.rcv_wnd >= 4096 * 4 {
                        logln5!("send ack");
                        self.send_ack();

                        self.ctl.rcv_wnd = 0;
                        self.rx_timer().disable();
                    } else {
                        logln5!("start rx timer");
                        self.rx_timer().start_with_timeout(150);
                    }
                } else {
                    logln5!("[ TCP ] Failed to store data");
                }
            } else {
                logln5!("tcp seq missmatch");
            }

            if hdr.flag_fin() {
                self.state = State::LastAck;
                self.send_ack_flags(TcpFlags::FIN);
                self.dc_timer().start_with_timeout(1000);
            }
        } else {
            logln5!("data empty and not flag fin");
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
                logln_disabled!("[ TCP ] Unexpected FinWait1 packet");
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
                logln_disabled!("[ TCP ] Unexpected FinWait2 packet");
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

    fn init_connected(&mut self, src_port: u16, packet: Packet<Tcp>) {
        self.set_src_port(src_port);

        let hdr = packet.header();

        self.ctl.iss = hdr.seq_nr();
        self.ctl.irs = hdr.seq_nr();

        self.setup_connection(packet);

        self.update_rcv_next(packet);

        logln5!("[ TCP ] Syn Received");

        self.state = State::SynReceived;
    }

    fn process_new_connection(&mut self, packet: Packet<Tcp>) -> Option<Arc<Socket>> {
        let hdr = packet.header();

        match (hdr.flag_syn(), hdr.flag_ack()) {
            (true, false) => {
                let sock = Socket::new_connected(self, packet);

                crate::kernel::net::tcp::register_handler(sock.clone());

                sock.ack_connection();

                return Some(sock);
            }
            _ => {}
        }
        None
    }

    fn process(&mut self, packet: Packet<Tcp>, in_buffer: &BufferQueue) {
        //print!(".");
        logln5!("process {:?}", self.state);
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
                logln5!(
                    "[ TCP ] Out of Order Packet Received {}, {} vs {}",
                    (hdr.seq_nr() - self.ctl.irs),
                    self.ctl.rcv_nxt,
                    hdr.seq_nr()
                );

                logln_disabled!("[ TCP ] Available buffer: {}", in_buffer.available_size());

                //self.send_ack();
                //self.send_ack();
                //self.send_ack_flags(TcpFlags::RST);
                //self.finalize();
                return;
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
                logln_disabled!("State not yet supported");
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

        logln_disabled!("[ TCP ] Connection closed");

        self.stop_timers();

        crate::kernel::net::tcp::release_handler(self.src_port as u32, self.target());

        let socket = self.socket();
        let in_buffer = socket.in_buffer();

        in_buffer.set_shutting_down(true);
        in_buffer.readers_queue().notify_all();
    }

    fn close(&mut self) {
        logln_disabled!("[ TCP ] Closing");
        match self.state {
            State::Established | State::SynReceived => {
                self.send_ack_flags(TcpFlags::FIN);

                self.state = State::FinWait1;
            }
            State::CloseWait => {
                logln_disabled!("[ TCP ] Close wait sent fin");
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

    pub fn listen(&mut self, _backlog: i32) {
        logln_disabled!("[ TCP ] Listening");
        self.state = State::Listen;
    }
}

struct SocketNewConnections {
    connections: Vec<Arc<Socket>>,
    backlog: i32,
}

impl SocketNewConnections {
    fn new() -> SocketNewConnections {
        SocketNewConnections {
            connections: Vec::new(),
            backlog: 0,
        }
    }

    fn init(&mut self, backlog: i32) {
        self.backlog = backlog;
        self.connections.reserve(backlog as usize);
    }
}

pub struct Socket {
    data: Mutex<SocketData>,
    connections: Spin<SocketNewConnections>,
    connections_wq: WaitQueue,

    in_buffer: BufferQueue,
    self_ref: Weak<Socket>,
}

impl Socket {
    pub fn new_unbound() -> Arc<Socket> {
        let sock = Arc::new_cyclic(|me| Socket {
            data: Mutex::new(SocketData::new_unbound(me)),
            in_buffer: BufferQueue::new_empty(true, true),
            connections: Spin::new(SocketNewConnections::new()),
            connections_wq: WaitQueue::new(),
            self_ref: me.clone(),
        });

        sock
    }

    fn new_connected(from: &SocketData, packet: Packet<Tcp>) -> Arc<Socket> {
        let sock = Arc::new_cyclic(|me| Socket {
            data: Mutex::new(SocketData::new_connected(from, packet, me)),
            in_buffer: BufferQueue::new_empty(true, true),
            connections: Spin::new(SocketNewConnections::new()),
            connections_wq: WaitQueue::new(),
            self_ref: me.clone(),
        });

        sock.in_buffer.init_size(4096 * 18);
        sock.init(false);

        sock
    }

    fn init(&self, listening: bool) {
        self.data.lock().init(listening);
    }

    fn in_buffer(&self) -> &BufferQueue {
        return &self.in_buffer;
    }

    pub fn ack_connection(&self) {
        self.data.lock().send_ack_flags(TcpFlags::SYN);
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

    pub fn listen(&self, backlog: i32) {
        self.data.lock().listen(backlog)
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

    pub fn me(&self) -> Arc<Socket> {
        self.self_ref.upgrade().unwrap()
    }

    fn read(&self, offset: usize, buf: &mut [u8], flags: MsgFlags) -> Result<usize> {
        Ok(self.in_buffer.read_data_from(
            offset,
            buf,
            flags.contains(MsgFlags::MSG_PEEK),
            WaitQueueFlags::empty(),
        )?)
    }

    fn update_window(&self) {
        self.data.lock().send_ack();
    }
}

impl INode for Socket {
    fn stat(&self) -> Result<Stat> {
        let mut stat = Stat::default();

        stat.st_mode.insert(syscall_defs::stat::Mode::IFSOCK);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXU);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXG);
        stat.st_mode.insert(syscall_defs::stat::Mode::IRWXO);

        Ok(stat)
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        //let data = self.data.lock();
        //if data.is_listening() {
        //    return Err(FsError::NotSupported);
        //}
        //drop(data);
        let r = self.read(0, buf, MsgFlags::empty())?;

        if r > 0 {
            self.update_window();
        }

        Ok(r)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        logln5!("write_at socket, len: {}", buf.len());
        let mut data = self.data.lock();

        if data.is_listening() {
            logln5!("write on listening socket");
            return Err(FsError::NotSupported);
        }

        if data.state == State::Established {
            if data.ctl.available_window() < buf.len() {
                let task = current_task();

                data.proxy_buffer.writers_queue().add_task(task.clone());

                //println!("[ TCP ] Proxy Buffer avail: {}", data.proxy_buffer.available_size());

                while data.proxy_buffer.available_size() < buf.len() {
                    if let Err(e) = WaitQueue::wait_lock(data) {
                        data = self.data.lock();

                        data.proxy_buffer.writers_queue().remove_task(task);

                        return Err(e)?;
                    }

                    data = self.data.lock();
                }

                data.proxy_buffer.try_append_data(buf);

                data.proxy_buffer.writers_queue().remove_task(task);
            } else {
                for o in (0..buf.len()).step_by(1460) {
                    let len = core::cmp::min(1460, buf.len() - o);

                    let mut packet = data.make_ack_packet(len, TcpFlags::empty());

                    packet.data_mut().copy_from_slice(&buf[o..o + len]);

                    data.send_packet(packet, true);

                    data.ctl.rcv_wnd = 0;
                    data.rx_timer().disable();
                }
            }
            Ok(buf.len())
        } else {
            logln5!("write before connected");
            Ok(data.proxy_buffer.append_data(buf)?)
        }
    }

    fn poll(
        &self,
        listen: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        let mut res = PollEventFlags::empty();

        let data = self.data.lock();

        if data.is_listening() {
            if let Some(pt) = listen {
                pt.listen(&self.connections_wq);
            }

            drop(data);

            if !self.connections.lock().connections.is_empty() {
                res.insert(PollEventFlags::READ);
            }

            return Ok(res);
        }

        if let Some(pt) = listen {
            if flags.contains(PollEventFlags::READ) {
                pt.listen(&self.in_buffer.readers_queue());
            }
            if flags.contains(PollEventFlags::WRITE) {
                pt.listen(&data.proxy_buffer.writers_queue());
            }
        }

        if (data.state == State::Closed || data.state == State::CloseWait)
            && !self.in_buffer.has_data()
        {
            res.insert(PollEventFlags::READ);
        } else {
            if self.in_buffer.has_data() {
                logln5!("in buffer has data");
                res.insert(PollEventFlags::READ);
            }
        }

        if data.state == State::Established {
            res.insert(PollEventFlags::WRITE);
        }

        Ok(res)
    }

    fn close(&self, _flags: OpenFlags) {
        //self.data.lock().close();
    }

    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize> {
        default_driver().ioctl(cmd, arg)
    }

    fn as_socket(&self) -> Option<Arc<dyn SocketService>> {
        logln5!("AS SOCKET???");
        Some(self.self_ref.upgrade()?)
    }
}

impl SocketService for Socket {
    fn listen(&self, backlog: i32) -> SyscallResult {
        self.init(true);

        crate::kernel::net::tcp::register_handler(self.me()).ok_or(SyscallError::EADDRINUSE)?;

        self.listen(backlog);

        Ok(0)
    }

    fn accept(
        &self,
        mut sock_addr: SockAddrPtr,
        _addrlen: Option<&mut u32>,
    ) -> core::result::Result<Arc<dyn SocketService>, SyscallError> {
        let mut lock = self
            .connections_wq
            .wait_lock_for(WaitQueueFlags::empty(), &self.connections, |l| {
                !l.connections.is_empty()
            })?
            .unwrap();

        if !sock_addr.is_null() {
            let addr = sock_addr.as_sock_addr_in_mut();
            *addr = SockAddrIn::new(self.dst_port(), self.target().into());
        }

        Ok(lock.connections.pop().unwrap())
    }

    fn bind(&self, sock_addr: SockAddrPtr, addrlen: u32) -> SyscallResult {
        logln5!("tcp bind");
        let sock_addr = sock_addr.as_sock_addr_in();
        if addrlen as usize != core::mem::size_of::<SockAddrIn>() {
            return Err(SyscallError::EINVAL);
        }
        self.set_src_port(sock_addr.port());

        Ok(0)
    }

    fn connect(&self, sock_addr: SockAddrPtr, addrlen: u32) -> SyscallResult {
        logln5!("tcp connect");
        let sock_addr = sock_addr.as_sock_addr_in();
        if addrlen as usize != core::mem::size_of::<SockAddrIn>() {
            return Err(SyscallError::EINVAL);
        }

        self.set_dst_port(sock_addr.port());
        self.set_target(sock_addr.sin_addr.s_addr.into());

        self.in_buffer.init_size(PAGE_SIZE * 18);
        self.init(false);

        crate::kernel::net::tcp::register_handler(self.me()).ok_or(SyscallError::EADDRINUSE)?;

        self.connect();

        Ok(0)
    }

    fn msg_send(&self, hdr: &MsgHdr, _flags: MsgFlags) -> SyscallResult {
        logln!("tcp msg_send");
        let iovecs = hdr.iovecs();

        let mut total = 0;

        for iovec in iovecs {
            total += self.write_at(0, iovec.get_bytes())?;
        }

        Ok(total)
    }

    fn msg_recv(&self, hdr: &mut MsgHdr, flags: MsgFlags) -> SyscallResult {
        let iovecs = hdr.iovecs_mut();

        let mut total = 0;

        for iovec in iovecs {
            if total > 0 && !self.in_buffer.has_data() {
                break;
            }

            let offset = if flags.contains(MsgFlags::MSG_PEEK) {
                total
            } else {
                0
            };

            let read = self.read(offset, iovec.get_bytes_mut(), flags)?;

            if read == 0 {
                return Ok(total);
            }

            total += read
        }

        if total > 0 && flags.contains(MsgFlags::MSG_PEEK) {
            self.update_window();
        }

        return Ok(total);
    }

    fn get_socket_option(
        &self,
        _layer: i32,
        _option: SockOption,
        _buffer: *mut (),
        _socklen: Option<&mut u32>,
    ) -> SyscallResult {
        logln5!("getsockopt {:?}", _option);
        Ok(0)
    }

    fn as_inode(&self) -> Option<Arc<dyn INode>> {
        Some(self.self_ref.upgrade()?)
    }
}

impl NetSocketService for Socket {
    fn process_packet(&self, packet: Packet<Ip>) {
        logln5!("process packet start");
        let mut data = self.data.lock();
        logln5!("process packet locked");

        if !data.is_listening() {
            data.process(packet.upgrade(), &self.in_buffer);

            if data.state == State::Closed {
                self.in_buffer.readers_queue().notify_all();
            }
        } else {
            if let Some(sock) = data.process_new_connection(packet.upgrade()) {
                self.connections.lock().connections.push(sock);
                self.connections_wq.notify_all();
            }
        }
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        println!("Failed to send to port {}", dst_port);

        self.data.lock().finalize();
    }

    fn src_port(&self) -> u32 {
        self.src_port() as u32
    }

    fn target(&self) -> Ip4 {
        self.target()
    }

    fn set_src_port(&self, src_port: u32) {
        self.set_src_port(src_port as u16);
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.data.lock().close();
        logln_disabled!("[ TCP ] Socket Removed");
    }
}
