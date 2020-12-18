mod reg;

use crate::arch::mm::virt::map_to_flags;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::mm::virt::PageFlags;
use alloc::sync::Arc;
use spin::Once;

use self::reg::*;

struct Ahci {}

impl PciDeviceHandle for Ahci {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2922) => true,
            _ => false,
        }
    }

    fn start(&self, _pci_data: &PciHeader) -> bool {
        println!("[ AHCI ] Ahci driver");

        if let PciHeader::Type0(dhdr) = _pci_data {
            println!("[ AHCI ] Base: {:0x}", dhdr.base_address5());

            let mapped = PhysAddr(dhdr.base_address5() as usize).to_virt();

            map_to_flags(
                mapped,
                PhysAddr(dhdr.base_address5() as usize),
                PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
            );

            //use crate::kernel::mm::virt::PageFlags;

            //crate::kernel::mm::map_flags(
            //    mapped,
            //    PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
            //);

            let hba = unsafe { mapped.read_ref::<HbaMem>() };

            println!("[ AHCI ] Ports: 0x{:b}", hba.pi());
            println!("[ AHCI ] Cap:   0b{:b}", hba.cap());
            println!("[ AHCI ] ghc  : 0x{:x}", hba.ghc());
            println!("[ AHCI ] vers : 0x{:x}", hba.vs());
            println!("[ AHCI ] cap2 : 0x{:x}", hba.cap2());
        }

        let data = _pci_data.hdr();
        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ AHCI ] Interrupt line: {}", p);
        }

        true
    }
}

static DEVICE: Once<Arc<Ahci>> = Once::new();

fn device() -> &'static Arc<Ahci> {
    DEVICE.get().unwrap()
}

fn init() {
    DEVICE.call_once(|| Arc::new(Ahci {}));

    register_pci_device(device().clone());
}

module_init!(init);
