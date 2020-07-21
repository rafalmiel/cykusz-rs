use alloc::sync::Arc;
use core::sync::atomic::{AtomicU16, Ordering};

use crate::arch::raw::mm::VirtAddr;
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::udp::{Udp, UdpService};
use crate::kernel::net::util::NetU16;
use crate::kernel::net::{
    default_driver, Packet, PacketDownHierarchy, PacketHeader, PacketKind, PacketUpHierarchy,
};
use crate::kernel::sched::current_task;
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;

#[derive(Debug, Copy, Clone)]
pub struct Dns {}

impl PacketKind for Dns {}

impl PacketUpHierarchy<Dns> for Packet<Udp> {}

impl PacketHeader<DnsHeader> for Packet<Dns> {}

#[repr(packed)]
struct DnsHeader {
    id: NetU16,
    flags: NetU16,
    question_cnt: NetU16,
    answer_cnt: NetU16,
    authority_cnt: NetU16,
    additional_cnt: NetU16,
}

impl DnsHeader {
    fn id(&self) -> u16 {
        self.id.value()
    }

    fn set_id(&mut self, id: u16) {
        self.id = NetU16::new(id);
    }

    fn flags(&self) -> u16 {
        self.flags.value()
    }

    fn set_flags(&mut self, flags: u16) {
        self.flags = NetU16::new(flags);
    }

    fn question_count(&self) -> u16 {
        self.question_cnt.value()
    }

    fn set_question_count(&mut self, cnt: u16) {
        self.question_cnt = NetU16::new(cnt);
    }

    fn answer_count(&self) -> u16 {
        self.answer_cnt.value()
    }

    fn set_answer_count(&mut self, cnt: u16) {
        self.answer_cnt = NetU16::new(cnt);
    }

    fn authority_count(&self) -> u16 {
        self.authority_cnt.value()
    }

    fn set_authority_count(&mut self, cnt: u16) {
        self.authority_cnt = NetU16::new(cnt);
    }

    fn additional_count(&self) -> u16 {
        self.additional_cnt.value()
    }

    fn set_additional_count(&mut self, cnt: u16) {
        self.additional_cnt = NetU16::new(cnt);
    }

    fn payload(&mut self) -> PostHeader {
        PostHeader {
            addr: VirtAddr(self as *const _ as usize + core::mem::size_of::<DnsHeader>()),
        }
    }
}

struct QName {
    addr: VirtAddr,
}

impl QName {
    const fn new(a: VirtAddr) -> QName {
        QName { addr: a }
    }

    fn skip(&self) -> VirtAddr {
        let mut a = self.addr;

        while unsafe { a.read::<u8>() != 0 } {
            a += 1;
        }

        a + 1
    }

    fn encode(&mut self, name: &[u8]) -> VirtAddr {
        let mut label = self.addr;

        let mut ptr = label + 1;
        let mut len = 0;

        for e in name {
            match *e as char {
                '.' => unsafe {
                    label.store(len as u8);
                    label = ptr;
                    len = 0;
                },
                a => unsafe {
                    ptr.store(a as u8);
                    len += 1;
                },
            }

            ptr += 1;
        }

        unsafe {
            label.store(len as u8);
            ptr.store(0 as u8);
            ptr += 1;
        }

        ptr
    }
}

struct PostHeader {
    addr: VirtAddr,
}

impl PostHeader {
    fn question(&mut self) -> Question {
        Question { addr: self.addr }
    }

    fn answer(&mut self) -> Answer {
        Answer { addr: self.addr }
    }
}

struct Question {
    addr: VirtAddr,
}

impl Question {
    fn as_postheader(&self) -> PostHeader {
        PostHeader { addr: self.addr }
    }

    fn skip(&self) -> Question {
        Question {
            addr: QName::new(self.addr).skip() + 2 + 2,
        }
    }

    fn encode_name(&mut self, name: &[u8]) -> Question {
        Question {
            addr: QName::new(self.addr).encode(name),
        }
    }

    fn encode_type(&mut self, typ: u16) -> Question {
        unsafe {
            self.addr.store(NetU16::new(typ));
        }

        Question {
            addr: self.addr + 2,
        }
    }

    fn encode_class(&mut self, class: u16) -> Question {
        unsafe {
            self.addr.store(NetU16::new(class));
        }

        Question {
            addr: self.addr + 2,
        }
    }
}

struct Answer {
    addr: VirtAddr,
}

impl Answer {
    fn skip_name(&self) -> VirtAddr {
        let f = unsafe { self.addr.read::<NetU16>() };

        if f.value() & 0xC000 > 0 {
            self.addr + 2
        } else {
            QName::new(self.addr).skip()
        }
    }

    fn rdata(&self) -> &[u8] {
        let mut a = self.skip_name() + 8;

        let len = unsafe { a.read::<NetU16>().value() };

        a += 2;

        unsafe { core::slice::from_raw_parts(a.0 as *const u8, len as usize) }
    }
}

static QUERY_ID: AtomicU16 = AtomicU16::new(0);

fn query_host(dns: Arc<DnsService>, host: &[u8], src_port: u32) -> Option<Ip4> {
    let drv = default_driver();

    let mut packet: Packet<Dns> =
        crate::kernel::net::udp::create_packet(src_port as u16, 53, 512, drv.dns()).upgrade();

    let header: &mut DnsHeader = packet.header_mut();

    let id = QUERY_ID.fetch_add(1, Ordering::SeqCst);

    header.set_id(id);
    header.set_flags(0x100);
    header.set_question_count(1);
    header.set_answer_count(0);
    header.set_authority_count(0);
    header.set_additional_count(0);

    let mut enc = header.payload().question();

    enc.encode_name(host).encode_type(1).encode_class(1);

    crate::kernel::net::udp::send_packet(packet.downgrade());

    Some(dns.await_result())
}

struct DnsService {
    ip_result: Spin<Option<Ip4>>,
    wait_queue: WaitQueue,
}

impl DnsService {
    fn new() -> DnsService {
        DnsService {
            ip_result: Spin::new(None),
            wait_queue: WaitQueue::new(),
        }
    }

    fn await_result(&self) -> Ip4 {
        let mut res = self.ip_result.lock();

        self.wait_queue.add_task(current_task());

        while res.is_none() {
            self.wait_queue.wait_lock(res);

            res = self.ip_result.lock();
        }

        self.wait_queue.remove_task(current_task());

        res.unwrap()
    }
}

impl UdpService for DnsService {
    fn process_packet(&self, packet: Packet<Udp>) {
        let mut packet: Packet<Dns> = packet.upgrade();

        let hdr = packet.header_mut();

        let mut phdr = hdr.payload();

        for _ in 0..hdr.question_count() {
            phdr = phdr.question().skip().as_postheader();
        }

        {
            if hdr.answer_count() > 0 {
                let ans = phdr.answer();

                let rdata = ans.rdata();

                *self.ip_result.lock() = Some(Ip4::new(rdata));
            } else {
                *self.ip_result.lock() = Some(Ip4::empty());
            }
        }

        self.wait_queue.notify_one();
    }

    fn port_unreachable(&self, port: u32, _dst_port: u32) {
        println!("DNS port unreachable");

        crate::kernel::net::udp::release_handler(port);
    }
}

pub fn get_ip_by_host(host: &[u8]) -> Option<Ip4> {
    let dns = Arc::new(DnsService::new());

    if let Some(port) = crate::kernel::net::udp::register_ephemeral_handler(dns.clone()) {
        let res = query_host(dns, host, port);

        crate::kernel::net::udp::release_handler(port);

        res
    } else {
        None
    }
}
