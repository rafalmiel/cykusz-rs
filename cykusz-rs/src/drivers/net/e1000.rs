use alloc::boxed::Box;

use crate::drivers::pci::PciData;
use crate::drivers::pci::PciDeviceHandle;

struct E1000 {}

impl PciDeviceHandle for E1000 {
    fn handles(&self, pci_dev_id: u64) -> bool {
        return match pci_dev_id {
            0x100E | 0x1502 => true,
            _ => false,
        };
    }

    fn start(&self, data: &PciData) {
        use crate::drivers::acpi::pci_map::get_irq_mapping;
        use crate::drivers::pci::read as pci_read;

        let pin = data.interrupt_pin();

        println!("get_irq_mapping {} {} {}", data.bus, data.dev, pin);
        let int_num = get_irq_mapping(data.bus as u32, data.dev as u32, (pin - 1) as u32).unwrap();

        let bar1 = pci_read(data.seg, data.bus, data.dev, data.fun, 0x10, 32);
        let bar2 = pci_read(data.seg, data.bus, data.dev, data.fun, 0x14, 32);
        let bar3 = pci_read(data.seg, data.bus, data.dev, data.fun, 0x18, 32);

        println!("Starting device E1000, pin: {} -> {}", pin - 1, int_num);

        println!("bar0: 0x{:x}, bar1: 0x{:x}, bar2: 0x{:x}", bar1, bar2, bar3);
    }
}

fn init() {
    crate::drivers::pci::register_pci_device(Box::new(E1000 {}));
}

module_init!(init);
