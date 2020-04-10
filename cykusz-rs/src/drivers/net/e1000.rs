use crate::drivers::pci::PciDevice;
use alloc::boxed::Box;

struct E1000 {

}

impl PciDevice for E1000 {
    fn handles(&self, pci_dev_id: u64) -> bool {
        return match pci_dev_id {
            0x100E => true,
            _ => false,
        }
    }

    fn start(&self) {
        println!("Starting device E1000");
    }
}

fn init() {
    crate::drivers::pci::register_pci_device(Box::new(E1000{}));
}

module_init!(init);