use alloc::collections::VecDeque;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use syscall_defs::net::{MsgFlags, MsgHdr, NetU16, SockAddr, SockOption};

use syscall_defs::poll::PollEventFlags;
use syscall_defs::{OpenFlags, SyscallError, SyscallResult};

use syscall_defs::stat::Stat;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::net::ip::{Ip, Ip4};
use crate::kernel::net::socket::SocketService;
use crate::kernel::net::udp::Udp;
use crate::kernel::net::{default_driver, Packet, PacketHeader, PacketTrait, PacketUpHierarchy};
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

struct RecvPacket {
    src_port: u32,
    src_ip: Ip4,
    data: Vec<u8>,
}

impl RecvPacket {
    fn new(src_port: u32, src_ip: Ip4, data: Vec<u8>) -> RecvPacket {
        RecvPacket {
            src_port,
            src_ip,
            data,
        }
    }

    fn src_port(&self) -> u32 {
        self.src_port
    }

    fn src_ip(&self) -> Ip4 {
        self.src_ip
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub struct Socket {
    buffer: Spin<VecDeque<RecvPacket>>,
    buffer_wq: WaitQueue,
    src_port: AtomicU32,
    dst_port: Spin<Option<u32>>,
    dst_ip: Spin<Option<Ip4>>,
    error: AtomicBool,
    self_ref: Weak<Socket>,
}

impl Socket {
    pub fn new_unbound() -> Arc<Socket> {
        Arc::new_cyclic(|me| Socket {
            buffer: Spin::new(VecDeque::new()),
            buffer_wq: WaitQueue::new(),
            src_port: AtomicU32::new(0),
            dst_port: Spin::new(None),
            dst_ip: Spin::new(None),
            error: AtomicBool::new(false),
            self_ref: me.clone(),
        })
    }

    pub fn set_dst_port(&self, port: u32) {
        *self.dst_port.lock() = Some(port);
    }

    pub fn dst_port(&self) -> Option<u32> {
        *self.dst_port.lock()
    }

    pub fn set_src_port(&self, port: u32) {
        self.src_port.store(port, Ordering::SeqCst);
    }

    pub fn src_port(&self) -> u32 {
        self.src_port.load(Ordering::SeqCst)
    }

    pub fn set_dst_ip(&self, ip: Ip4) {
        *self.dst_ip.lock() = Some(ip)
    }

    pub fn dst_ip(&self) -> Option<Ip4> {
        *self.dst_ip.lock()
    }

    pub fn me(&self) -> Arc<Socket> {
        self.self_ref.upgrade().unwrap()
    }

    fn send(&self, buf: &[u8], target: Option<(u32, Ip4)>) -> Result<usize> {
        let (dst_port, dst_ip) = if let Some((port, ip)) = target {
            (port, ip)
        } else {
            (
                self.dst_port().unwrap_or(0),
                self.dst_ip().unwrap_or(Ip4::empty()),
            )
        };

        if self.src_port() == 0 {
            crate::kernel::net::udp::register_handler(self.me());
        }

        if dst_port == 0 || dst_ip.is_empty() {
            return Err(FsError::NotSupported);
        }

        let mut packet = crate::kernel::net::udp::create_packet(
            self.src_port() as u16,
            dst_port as u16,
            buf.len(),
            dst_ip,
        );

        let dest_buf = packet.data_mut();

        let amount = core::cmp::min(dest_buf.len(), buf.len());

        dest_buf[..amount].copy_from_slice(&buf[..amount]);

        crate::kernel::net::udp::send_packet(packet);

        Ok(amount)
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
        logln4!("udp read {}", buf.len());
        let mut data = self
            .buffer_wq
            .wait_lock_for(WaitQueueFlags::empty(), &self.buffer, |l| !l.is_empty())?
            .unwrap();

        let packet = data.pop_front().unwrap();

        drop(data);

        let size = core::cmp::min(buf.len(), packet.data().len());

        buf[..size].copy_from_slice(&packet.data().as_slice()[..size]);

        Ok(size)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        self.send(buf, None)
    }

    fn poll(
        &self,
        listen: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags> {
        if flags.is_empty() {
            return Ok(PollEventFlags::empty());
        }

        if let Some(p) = listen {
            p.listen(&self.buffer_wq);
        }

        let mut ret = PollEventFlags::empty();

        if flags.contains(PollEventFlags::WRITE) {
            ret.insert(PollEventFlags::WRITE);
        }

        if flags.contains(PollEventFlags::READ) {
            let has_data = !self.buffer.lock().is_empty();

            if has_data {
                ret.insert(PollEventFlags::READ)
            }
        }

        Ok(ret)
    }

    fn close(&self, _flags: OpenFlags) {
        crate::kernel::net::udp::release_handler(self.src_port());
    }

    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize> {
        default_driver().ioctl(cmd, arg)
    }

    fn as_socket(&self) -> Option<Arc<dyn SocketService>> {
        Some(self.self_ref.upgrade()?)
    }
}

impl SocketService for Socket {
    fn process_packet(&self, ip: Packet<Ip>) {
        let packet: Packet<Udp> = ip.upgrade();

        let ip_header = ip.header();
        let udp_header = packet.header();

        let recv = RecvPacket::new(
            udp_header.src_port.value() as u32,
            ip_header.src_ip,
            Vec::from(packet.data()),
        );

        self.buffer.lock().push_back(recv);

        self.buffer_wq.notify_all();
    }

    fn port_unreachable(&self, _port: u32, dst_port: u32) {
        logln4!("UDP: Failed to send to port {}", dst_port);
    }

    fn bind(&self, sock_addr: &SockAddr, addrlen: u32) -> SyscallResult {
        if addrlen as usize != core::mem::size_of::<SockAddr>() {
            return Err(SyscallError::EINVAL);
        }

        self.set_src_port(sock_addr.as_sock_addr_in().port() as u32);

        crate::kernel::net::udp::register_handler(self.me());

        return Ok(0);
    }

    fn connect(&self, sock_addr: &SockAddr, addrlen: u32) -> SyscallResult {
        if addrlen as usize != core::mem::size_of::<SockAddr>() {
            return Err(SyscallError::EINVAL);
        }

        let addr_in = sock_addr.as_sock_addr_in();

        self.set_dst_port(addr_in.port() as u32);
        self.set_dst_ip(addr_in.sin_addr.s_addr.into());

        if self.src_port() == 0 {
            crate::kernel::net::udp::register_handler(self.me());
        }

        Ok(0)
    }

    fn msg_send(&self, hdr: &MsgHdr, _flags: MsgFlags) -> SyscallResult {
        logln5!("UDP msg_send???");
        let dest = if let Some(addr) = hdr.sock_addr() {
            let addr_in = addr.as_sock_addr_in();
            Some((addr_in.port() as u32, addr_in.sin_addr.s_addr.into()))
        } else {
            None
        };

        let buf = hdr.iovecs();

        if buf.len() == 1 {
            Ok(self.send(buf[0].get_bytes(), dest)?)
        } else {
            let data = hdr
                .iovecs()
                .iter()
                .flat_map(|e| e.get_bytes())
                .copied()
                .collect::<Vec<_>>();

            Ok(self.send(data.as_slice(), dest)?)
        }
    }

    fn msg_recv(&self, hdr: &mut MsgHdr, _flags: MsgFlags) -> SyscallResult {
        logln5!("UDP msg_recv???");
        let mut data = self
            .buffer_wq
            .wait_lock_for(WaitQueueFlags::empty(), &self.buffer, |l| !l.is_empty())?
            .unwrap();

        let packet = data.pop_front().unwrap();

        drop(data);

        if let Some(addr) = hdr.sock_addr_mut() {
            let addr = addr.as_sock_addr_in_mut();
            addr.sin_port = NetU16::new(packet.src_port() as u16);
            addr.sin_addr.s_addr = packet.src_ip().into();
        }

        let mut offset = 0;

        Ok(hdr
            .iovecs_mut()
            .iter_mut()
            .map(|iovec| {
                let iovec = iovec.get_bytes_mut();
                let size = core::cmp::min(iovec.len(), packet.data().len() - offset);
                iovec[..size].copy_from_slice(&packet.data()[offset..offset + size]);
                offset += size;
                size
            })
            .sum::<usize>())
    }

    fn src_port(&self) -> u32 {
        self.src_port()
    }

    fn target(&self) -> Ip4 {
        self.dst_ip().unwrap()
    }

    fn set_src_port(&self, src_port: u32) {
        self.set_src_port(src_port);
    }

    fn set_socket_option(
        &self,
        _layer: i32,
        _option: SockOption,
        _buffer: *const (),
        _size: u32,
    ) -> SyscallResult {
        Ok(0)
    }

    fn as_inode(&self) -> Option<Arc<dyn INode>> {
        Some(self.self_ref.upgrade()?)
    }
}
