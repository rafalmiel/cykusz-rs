#![allow(dead_code)]

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::fs::vfs::FsError;
use crate::kernel::mm::VirtAddr;
use crate::kernel::net::eth::Eth;
use crate::kernel::net::ip::Ip4;
use crate::kernel::sched::create_param_task;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::RwSpin;
use syscall_defs::ioctl::net::{IfConf, IfReq, IfrFlags};
use syscall_defs::net::SockDomain;

pub use self::packet::*;

pub mod arp;
pub mod dhcp;
pub mod dns;
pub mod eth;
pub mod icmp;
pub mod ip;
pub mod packet;
pub mod socket;
pub mod tcp;
pub mod udp;
pub mod unix;
pub mod util;

pub trait NetDriver: Sync + Send {
    fn send(&self, packet: Packet<Eth>) -> bool;
    fn receive(&self) -> SignalResult<RecvPacket>;
    fn receive_finished(&self, id: usize);
    fn alloc_packet(&self, size: usize) -> Packet<Eth>;
    fn dealloc_patket(&self, packet: Packet<Eth>);
    fn read_mac(&self, mac: &mut [u8]);
    fn get_mac(&self) -> [u8; 6];
    fn link_up(&self);
}

struct NetDeviceData {
    pub ip: Ip4,
    pub default_gateway: Ip4,
    pub subnet: Ip4,
    pub dns: Ip4,
    pub dns_name: String,
}

impl NetDeviceData {
    fn configure(&mut self, ip: Ip4, default_gw: Ip4, subnet: Ip4, dns: Ip4) {
        self.ip = ip;
        self.default_gateway = default_gw;
        self.subnet = subnet;
        self.dns = dns;
    }
}

struct NetDevice {
    pub driver: Arc<dyn NetDriver>,
    pub data: RwSpin<NetDeviceData>,
}

impl NetDevice {
    fn new(driver: Arc<dyn NetDriver>) -> NetDevice {
        NetDevice {
            driver,
            data: RwSpin::new(NetDeviceData {
                ip: Ip4::empty(),
                default_gateway: Ip4::empty(),
                subnet: Ip4::empty(),
                dns: Ip4::empty(),
                dns_name: String::new(),
            }),
        }
    }

    pub fn configure(&self, ip: Ip4, default_gw: Ip4, subnet: Ip4, dns: Ip4) {
        self.data.write().configure(ip, default_gw, subnet, dns);
    }

    pub fn ip(&self) -> Ip4 {
        self.data.read().ip
    }

    pub fn default_gateway(&self) -> Ip4 {
        self.data.read().default_gateway
    }

    pub fn subnet(&self) -> Ip4 {
        self.data.read().subnet
    }

    pub fn dns(&self) -> Ip4 {
        self.data.read().dns
    }

    pub fn dns_name(&self) -> String {
        self.data.read().dns_name.clone()
    }

    pub fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize, FsError> {
        match cmd {
            syscall_defs::ioctl::net::SIOCGIFCONF => {
                let ifreq = unsafe { VirtAddr(arg).read_mut::<IfConf>() };
                ifreq.ifc_len = core::mem::size_of::<IfReq>() as i32;
                let buf = ifreq.get_req_array().ok_or(FsError::InvalidParam)?;

                let elem = &mut buf[0];

                elem.ifr_name.fill(0);
                elem.ifr_name[..4].copy_from_slice(b"eth0");

                let addr = unsafe { elem.ifrequ.ifr_addr.as_sock_addr_in_mut() };
                addr.sin_family = SockDomain::AfInet;
                addr.sin_addr.s_addr = self.ip().into();

                Ok(0)
            }
            syscall_defs::ioctl::net::SIOCGIFFLAGS => {
                let ifreq = unsafe { VirtAddr(arg).read_mut::<IfReq>() };
                ifreq.ifrequ.ifr_flags = IfrFlags::IFF_UP;
                Ok(0)
            }
            _ => Err(FsError::NoTty),
        }
    }
}

static DRIVERS: RwSpin<Vec<Arc<NetDevice>>> = RwSpin::new(Vec::new());
static DEFAULT_DRIVER: RwSpin<Option<Arc<NetDevice>>> = RwSpin::new(None);

fn default_driver() -> Arc<NetDevice> {
    DEFAULT_DRIVER.read().as_ref().unwrap().clone()
}

fn recv_thread(driver: usize) {
    let driver = DRIVERS.read()[driver].driver.clone();

    loop {
        let packet = driver
            .receive()
            .expect("[ NET ] Unexpected signal in recv_thread");

        process_packet(&packet);

        driver.receive_finished(packet.id);
    }
}

fn process_packet(packet: &RecvPacket) {
    eth::process_packet(packet.packet);
}

pub fn register_net_driver(driver: Arc<dyn NetDriver>) {
    let mut drivers = DRIVERS.write();

    let dev = Arc::new(NetDevice::new(driver));

    drivers.push(dev.clone());

    let mut def = DEFAULT_DRIVER.write();

    if def.is_none() {
        *def = Some(dev);
    }

    create_param_task(recv_thread as usize, drivers.len() - 1);
}

pub fn init() {
    unix::init();
    let def = DEFAULT_DRIVER.write();
    if let Some(dev) = &*def {
        arp::init();

        dev.driver.link_up();

        core::mem::drop(def);

        dhcp::init();
    }
}
