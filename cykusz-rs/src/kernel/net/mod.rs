#![allow(dead_code)]

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::mm::VirtAddr;
use crate::kernel::net::ip::Ip;
use crate::kernel::sched::create_param_task;
use crate::kernel::sync::RwSpin;

pub mod arp;
pub mod dhcp;
pub mod eth;
pub mod icmp;
pub mod ip;
pub mod udp;
pub mod util;

pub trait NetDriver: Sync + Send {
    fn send(&self, packet: Packet) -> bool;
    fn receive(&self) -> RecvPacket;
    fn receive_finished(&self, id: usize);
    fn alloc_packet(&self, size: usize) -> Packet;
    fn read_mac(&self, mac: &mut [u8]);
    fn get_mac(&self) -> [u8; 6];
    fn link_up(&self);
}

struct NetDeviceData {
    pub ip: Ip,
    pub default_gateway: Ip,
    pub subnet: Ip,
    pub dns: Ip,
    pub dns_name: String,
}

impl NetDeviceData {
    fn configure(&mut self, ip: Ip, default_gw: Ip, subnet: Ip, dns: Ip) {
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
                ip: Ip::empty(),
                default_gateway: Ip::empty(),
                subnet: Ip::empty(),
                dns: Ip::empty(),
                dns_name: String::new(),
            }),
        }
    }

    pub fn configure(&self, ip: Ip, default_gw: Ip, subnet: Ip, dns: Ip) {
        self.data.write().configure(ip, default_gw, subnet, dns);
    }

    pub fn ip(&self) -> Ip {
        self.data.read().ip
    }

    pub fn default_gateway(&self) -> Ip {
        self.data.read().default_gateway
    }

    pub fn subnet(&self) -> Ip {
        self.data.read().subnet
    }

    pub fn dns(&self) -> Ip {
        self.data.read().dns
    }

    pub fn dns_name(&self) -> String {
        self.data.read().dns_name.clone()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RecvPacket {
    pub packet: Packet,
    pub id: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct Packet {
    pub addr: VirtAddr,
    pub len: usize,
}

static DRIVERS: RwSpin<Vec<Arc<NetDevice>>> = RwSpin::new(Vec::new());
static DEFAULT_DRIVER: RwSpin<Option<Arc<NetDevice>>> = RwSpin::new(None);

fn default_driver() -> Arc<NetDevice> {
    DEFAULT_DRIVER.read().as_ref().unwrap().clone()
}

fn recv_thread(driver: usize) {
    let driver = DRIVERS.read()[driver].driver.clone();

    loop {
        let packet = driver.receive();

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
    let def = DEFAULT_DRIVER.write();
    if def.is_some() {
        arp::init();

        def.as_ref().unwrap().driver.link_up();

        core::mem::drop(def);

        crate::kernel::net::dhcp::send_discovery();
    }
}
