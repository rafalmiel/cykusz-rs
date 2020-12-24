mod reg;

use crate::arch::mm::virt::map_to_flags;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::mm::virt::PageFlags;
use alloc::sync::Arc;
use spin::Once;

use self::reg::*;
use bit_field::BitField;

struct Ahci {}

impl PciDeviceHandle for Ahci {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2922) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        println!("[ AHCI ] Ahci driver");

        if let PciHeader::Type0(dhdr) = pci_data {
            println!("[ AHCI ] Base: {:0x}", dhdr.base_address5());

            let mut mapped = PhysAddr(dhdr.base_address5() as usize).to_virt();

            map_to_flags(
                mapped,
                PhysAddr(dhdr.base_address5() as usize),
                PageFlags::NO_CACHE | PageFlags::WRT_THROUGH | PageFlags::WRITABLE,
            );

            //use crate::kernel::mm::virt::PageFlags;

            //crate::kernel::mm::map_flags(
            //    mapped,
            //    PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::WRT_THROUGH,
            //);

            let hba = unsafe { mapped.read_mut::<HbaMem>() };

            //hba.set_ghc(hba.ghc() | HbaMemGhcReg::IE);

            println!("[ AHCI ] Ports: 0b{:b}", hba.pi());
            println!("[ AHCI ] Cap:   0b{:b}", hba.cap());
            println!("[ AHCI ] ghc  : 0x{:x}", hba.ghc());
            println!("[ AHCI ] vers : 0x{:x}", hba.vs());
            println!("[ AHCI ] cap2 : 0x{:x}", hba.cap2());

            let pi = hba.pi();

            for i in 0..32 {
                if pi.get_bit(i) {
                    let port = hba.port_mut(i);

                    println!("[ AHCI ] Port {} fb: {}", i, port.fb());
                    println!("[ AHCI ] Port {} cb: {}", i, port.clb());

                    let sts = port.ssts();
                    let ipm = sts.interface_power_management();

                    let dev = sts.device_detection();

                    if let HbaPortSstsRegDet::PresentAndE = dev {
                        println!("Dev present and enabled");
                    }

                    if let HbaPortSstsRegIpm::Active = ipm {
                        println!("Dev active");
                    }

                    println!("sig: {:?}", port.sig().dev());

                    println!("Port cmd: 0x{:x}", port.cmd().bits());

                    port.set_cmd(port.cmd() | (HbaPortCmdReg::ST | HbaPortCmdReg::FRE));

                    println!("Port cmd started: 0x{:x}", port.cmd().bits());
                }
            }
        }

        let data = pci_data.hdr();
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
