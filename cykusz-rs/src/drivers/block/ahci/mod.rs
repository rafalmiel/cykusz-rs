use alloc::sync::Arc;

use spin::Once;

use crate::drivers::block::ahci::device::AhciDevice;
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::sync::Spin;

mod device;
mod port;
mod reg;

struct Ahci {
    dev: Spin<AhciDevice>,
}

fn ahci_handler() {
    device().dev.lock_irq().handle_interrupt();
}

fn sh_ahci_handler() -> bool {
    device().dev.lock_irq().handle_interrupt()
}

impl Ahci {
    pub fn new() -> Ahci {
        Ahci {
            dev: Spin::new(AhciDevice::new()),
        }
    }
}

impl PciDeviceHandle for Ahci {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2922) => true,
            (0x8086, 0x2829) => true,
            (0x8086, 0x1c03) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        device().dev.lock_irq().start(pci_data)
    }
}

static DEVICE: Once<Arc<Ahci>> = Once::new();

fn device() -> &'static Arc<Ahci> {
    unsafe { DEVICE.get_unchecked() }
}

fn init() {
    DEVICE.call_once(|| Arc::new(Ahci::new()));

    register_pci_device(device().clone());
}

module_init!(init);
