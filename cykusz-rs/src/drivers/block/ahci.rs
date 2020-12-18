use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use alloc::sync::Arc;
use spin::Once;
use crate::arch::raw::mm::PhysAddr;

#[repr(C, packed)]
struct HbaMem {
    cap: u32,
    ghc: u32,
    is: u32,
    pi: u32,
    vs: u32,
    ccc_ctl: u32,
    ccc_pts: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: u32,
    bohc: u32,
    _rsv: [u8; 0xa0 - 0x2c],
    vendor: [u8; 0x100 - 0xa0],
}

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

            let hba = unsafe {
                PhysAddr(dhdr.base_address5() as usize).to_mapped().read_ref::<HbaMem>()
            };

            println!("[ AHCI ] Ports: 0x{:b}", hba.pi);
            println!("[ AHCI ] Cap:   0b{:b}", hba.cap);
            println!("[ AHCI ] ghc  : 0x{:x}", hba.ghc);
            println!("[ AHCI ] vers : 0x{:x}", hba.vs);
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
