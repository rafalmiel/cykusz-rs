#![allow(dead_code)]

use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Once;

use addr::Addr;

use crate::arch::raw::mm::VirtAddr;
use crate::drivers::pci::{PciDeviceHandle, PciHeader};
use crate::kernel::sync::Spin;

mod addr;
mod device;
mod regs;

pub mod test;

#[allow(dead_code)]
struct E1000 {
    data: Spin<device::E1000Data>,
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

        data.read_mac();

        data.detect_eeprom();

        data.clear_filters();

        data.enable_interrupt();
        data.init_tx();
        data.init_rx();

        data.wait_link_up();

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
            }),
        })
    });

    crate::drivers::pci::register_pci_device(device().clone());
}

module_init!(init);
