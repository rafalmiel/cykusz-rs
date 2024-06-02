#![allow(dead_code)]

use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;

use addr::Addr;

use crate::drivers::net::e1000::device::E1000_NUM_TX_DESCS;
use crate::drivers::pci::{PciDeviceHandle, PciHeader};
use crate::kernel::mm::MappedAddr;
use crate::kernel::net::eth::Eth;
use crate::kernel::net::{NetDriver, Packet, RecvPacket};
use crate::kernel::sched::current_task;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::{LockApi, Spin};
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

    fn receive(&self) -> SignalResult<RecvPacket> {
        let task = current_task();

        self.rx_wqueue.add_task(task.clone());

        loop {
            let mut data = self.data.lock_irq();

            if let Some(p) = data.receive() {
                self.rx_wqueue.remove_task(task);
                return Ok(p);
            } else {
                WaitQueue::wait_lock(data)?
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

    fn dealloc_patket(&self, packet: Packet<Eth>) {
        let data = self.data.lock_irq();

        data.dealloc_packet(packet);
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

        data.link_up();

        data.wait_link_up();
    }
}

impl PciDeviceHandle for E1000 {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        if pci_vendor_id == 0x8086 {
            return match pci_dev_id {
                0x100E | // qemu e1000
                0x10d3 | // qemu e1000e
                0x100f | // vbox e1000
                0x1004 | // vbox e1000
                0x1502   // T-420
                    => true,
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
    fn handle_irq(&self) -> bool {
        self.data.lock_irq().handle_irq()
    }

    fn msi_handle_rq0(&self) {
        self.data.lock_irq().handle_rq0()
    }
}

static DEVICE: Once<Arc<E1000>> = Once::new();

fn device() -> &'static Arc<E1000> {
    DEVICE.get().unwrap()
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
                ring_buf: MappedAddr(0),
                tx_pkts: [None; E1000_NUM_TX_DESCS],
            }),
            rx_wqueue: WaitQueue::new(),
        })
    });

    crate::drivers::pci::register_pci_device(device().clone());
}

module_init!(init);
