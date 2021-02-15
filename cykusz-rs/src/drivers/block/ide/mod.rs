#![allow(dead_code)]

use alloc::sync::Arc;

use spin::Once;

use crate::drivers::block::ide::drive::IdeDevice;
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::sync::Spin;

mod channel;
mod drive;

struct Ide {
    dev: Spin<IdeDevice>,
}

fn ata_handler() -> bool {
    device().dev.lock_irq().handle_interrupt()
}

impl Ide {
    pub fn new() -> Ide {
        Ide {
            dev: Spin::new(IdeDevice::new()),
        }
    }
}

impl PciDeviceHandle for Ide {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x7010) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        self.dev.lock_irq().start(pci_data)
    }
}

static DEVICE: Once<Arc<Ide>> = Once::new();

fn device() -> &'static Arc<Ide> {
    DEVICE.get().unwrap()
}

fn init() {
    DEVICE.call_once(|| Arc::new(Ide::new()));

    register_pci_device(device().clone());
}

module_init!(init);
