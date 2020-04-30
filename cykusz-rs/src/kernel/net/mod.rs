use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::mm::PhysAddr;
use crate::kernel::sched::create_param_task;
use crate::kernel::sync::RwSpin;

pub trait NetDriver: Sync + Send {
    fn send(&self, packet: &[u8]) -> bool;
    fn receive(&self) -> Packet;
    fn receive_finished(&self, id: usize);
}

pub struct Packet {
    pub addr: PhysAddr,
    pub len: usize,
    pub id: usize,
}

static DRIVERS: RwSpin<Vec<Arc<dyn NetDriver>>> = RwSpin::new(Vec::new());

fn recv_thread(driver: usize) {
    let driver = DRIVERS.read()[driver].clone();

    loop {
        let packet = driver.receive();

        process_packet(&packet);

        driver.receive_finished(packet.id);
    }
}

fn process_packet(packet: &Packet) {
    println!("Packet Received {}", packet.id);
}

pub fn register_net_driver(driver: Arc<dyn NetDriver>) {
    let mut drivers = DRIVERS.write();

    drivers.push(driver);

    create_param_task(recv_thread as usize, drivers.len() - 1);
}

pub fn init() {}
