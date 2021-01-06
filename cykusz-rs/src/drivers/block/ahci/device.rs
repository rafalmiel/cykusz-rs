use alloc::string::String;
use alloc::sync::Arc;

use bit_field::BitField;

use crate::arch::idt::add_shared_irq_handler;
use crate::arch::int::{set_active_high, set_irq_dest};
use crate::drivers::block::ahci::port::Port;
use crate::drivers::block::ahci::reg::*;
use crate::drivers::pci::PciHeader;
use crate::kernel::block::{register_blkdev, BlockDevice};
use crate::kernel::mm::map_to_flags;
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::VirtAddr;

pub struct AhciDevice {
    ports: [Option<Arc<Port>>; 32],
    hba: VirtAddr,
}

impl AhciDevice {
    pub fn new() -> AhciDevice {
        AhciDevice {
            ports: [None; 32],
            hba: VirtAddr(0),
        }
    }

    fn hba(&mut self) -> &mut HbaMem {
        unsafe { self.hba.read_mut::<HbaMem>() }
    }

    fn enable_interrupts(&mut self, pci_data: &PciHeader) {
        pci_data.hdr().enable_bus_mastering();

        let data = pci_data.hdr();
        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ AHCI ] Using interrupt: {}", p);

            set_irq_dest(p as u8, p as u8 + 32);
            set_active_high(p as u8, true);
            add_shared_irq_handler(p as usize + 32, super::ahci_handler);
        }
    }

    fn start_hba(&mut self) -> bool {
        use crate::alloc::string::ToString;

        let mut hba = self.hba();

        hba.set_ghc(hba.ghc() | HbaMemGhcReg::IE);

        let pi = hba.pi();

        let mut disk_nr: u32 = 1;

        for i in 0..32 {
            if pi.get_bit(i) {
                let port = hba.port_mut(i);
                if port.probe(i) {
                    let addr = VirtAddr(port as *const _ as usize);
                    drop(port);
                    drop(hba);

                    let port_dev = Arc::new(Port::new(addr));

                    if let Err(d) = register_blkdev(BlockDevice::new(
                        String::from("disk") + &disk_nr.to_string(),
                        port_dev.clone(),
                    )) {
                        panic!("Failed to register blkdev {:?}", d);
                    }

                    disk_nr += 1;

                    self.ports[i] = Some(port_dev);

                    hba = self.hba();
                }
            }
        }

        true
    }

    pub fn start(&mut self, pci_data: &PciHeader) -> bool {
        if let PciHeader::Type0(dhdr) = pci_data {
            self.hba = PhysAddr(dhdr.base_address5() as usize).to_virt();

            map_to_flags(
                self.hba,
                PhysAddr(dhdr.base_address5() as usize),
                PageFlags::NO_CACHE | PageFlags::WRT_THROUGH | PageFlags::WRITABLE,
            );

            self.enable_interrupts(pci_data);

            self.start_hba();

            return true;
        }

        false
    }

    pub fn handle_interrupt(&mut self) -> bool {
        let hba = self.hba();

        if hba.is() != 0 {
            hba.set_is(hba.is());

            if let Some(p) = &self.ports[0] {
                p.handle_interrupt();
            }
        }

        return false;
    }
}
