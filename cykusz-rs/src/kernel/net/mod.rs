use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::mm::{MappedAddr, VirtAddr};
use crate::kernel::sched::create_param_task;
use crate::kernel::sync::RwSpin;

pub mod arp;
pub mod dhcp;
pub mod eth;
pub mod ip;
pub mod udp;
pub mod util;

pub trait NetDriver: Sync + Send {
    fn send(&self, packet: Packet) -> bool;
    fn receive(&self) -> RecvPacket;
    fn receive_finished(&self, id: usize);
    fn alloc_packet(&self, size: usize) -> Packet;
    fn get_mac(&self, mac: &mut [u8]);
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

static DRIVERS: RwSpin<Vec<Arc<dyn NetDriver>>> = RwSpin::new(Vec::new());
static DEFAULT_DRIVER: RwSpin<Option<Arc<dyn NetDriver>>> = RwSpin::new(None);

fn default_driver() -> Arc<dyn NetDriver> {
    DEFAULT_DRIVER.read().as_ref().unwrap().clone()
}

fn recv_thread(driver: usize) {
    let driver = DRIVERS.read()[driver].clone();

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

    drivers.push(driver.clone());

    let mut def = DEFAULT_DRIVER.write();

    if def.is_none() {
        *def = Some(driver);
    }

    create_param_task(recv_thread as usize, drivers.len() - 1);
}

pub fn init() {}
