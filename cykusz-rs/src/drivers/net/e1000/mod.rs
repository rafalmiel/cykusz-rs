#![allow(dead_code)]

use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;

use addr::Addr;

use crate::arch::raw::mm::VirtAddr;
use crate::drivers::pci::{PciDeviceHandle, PciHeader};
use crate::kernel::net::eth::Eth;
use crate::kernel::net::{NetDriver, Packet, RecvPacket};
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;

mod addr;
mod device;
mod regs;

pub mod test;

#[allow(dead_code)]
struct E1000 {
    data: Spin<device::E1000Data>,
    rx_wqueue: WaitQueue,
}

impl NetDriver for E1000 {
    fn send(&self, packet: Packet<Eth>) -> bool {
        self.data.lock_irq().send(packet);

        true
    }

    fn receive(&self) -> RecvPacket {
        loop {
            let mut data = self.data.lock_irq();

            if let Some(p) = data.receive() {
                return p;
            } else {
                core::mem::drop(data);

                self.rx_wqueue.wait();
            }
        }
    }

    fn receive_finished(&self, id: usize) {
        let mut data = self.data.lock_irq();

        data.receive_finished(id);
    }

    fn alloc_packet(&self, size: usize) -> Packet<Eth> {
        let data = self.data.lock_irq();

        data.alloc_packet(size)
    }

    fn read_mac(&self, mac: &mut [u8]) {
        let data = self.data.lock_irq();

        data.read_mac(mac)
    }

    fn get_mac(&self) -> [u8; 6] {
        let data = self.data.lock_irq();

        data.get_mac()
    }

    fn link_up(&self) {
        let mut data = self.data.lock_irq();

        data.enable_interrupt();

        data.wait_link_up();
    }
}

impl PciDeviceHandle for E1000 {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        if pci_vendor_id == 0x8086 {
            return match pci_dev_id {
                0x100E | 0x1502 => true,
                _ => false,
            };
        }

        false
    }

    fn start(&self, header: &PciHeader) -> bool {
        header.hdr().enable_bus_mastering();

        let mut data = self.data.lock_irq();

        data.init(header);

        data.reset();

        data.init_mac();

        data.detect_eeprom();

        data.clear_filters();

        data.init_tx();
        data.init_rx();

        crate::kernel::net::register_net_driver(device().clone());

        true
    }
}

impl E1000 {
    fn handle_irq(&self) {
        self.data.lock_irq().handle_irq();
    }
}

static DEVICE: Once<Arc<E1000>> = Once::new();

fn device() -> &'static Arc<E1000> {
    DEVICE.r#try().unwrap()
}

fn init() {
    DEVICE.call_once(|| {
        Arc::new(E1000 {
            data: Spin::new(device::E1000Data {
                hdr: None,
                addr: Addr::new(),
                int_nr: 0,
                has_eeprom: false,
                mac: [0; 6],
                rx_ring: Vec::new(),
                tx_ring: Vec::new(),
                rx_cur: 0,
                tx_cur: 0,
                ring_buf: VirtAddr(0),
            }),
            rx_wqueue: WaitQueue::new(),
        })
    });

    crate::drivers::pci::register_pci_device(device().clone());
}

module_init!(init);
