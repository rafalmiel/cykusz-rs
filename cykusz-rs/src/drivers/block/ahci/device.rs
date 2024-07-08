use alloc::string::String;
use alloc::sync::Arc;
use bit_field::BitField;

use crate::drivers::block::ahci::port::Port;
use crate::drivers::block::ahci::reg::*;
use crate::drivers::pci::PciHeader;
use crate::kernel::block::{register_blkdev, BlockDevice};
use crate::kernel::mm::VirtAddr;

pub struct AhciDevice {
    ports: [Option<Arc<Port>>; 32],
    hba: VirtAddr,
}

impl AhciDevice {
    pub fn new() -> AhciDevice {
        const EMPTY: Option<Arc<Port>> = None;
        AhciDevice {
            ports: [EMPTY; 32],
            hba: VirtAddr(0),
        }
    }

    fn hba(&mut self) -> &mut HbaMem {
        unsafe { self.hba.read_mut::<HbaMem>() }
    }

    fn enable_interrupts(&mut self, pci_data: &PciHeader) {
        let mut is_msi = true;
        if let Some(int) = pci_data
            .enable_msi_interrupt(super::ahci_handler)
            .or_else(|| {
                is_msi = false;
                pci_data.enable_pci_interrupt(super::sh_ahci_handler)
            })
        {
            logln!(
                "[ AHCI ] Using {} interrupt: {}",
                if is_msi { "MSI" } else { "PCI" },
                int
            );
            println!(
                "[ AHCI ] Using {} interrupt: {}",
                if is_msi { "MSI" } else { "PCI" },
                int
            );
        }
    }

    fn start_hba(&mut self) -> bool {
        use crate::alloc::string::ToString;

        let mut hba = self.hba();
        //println!("{:?}", hba.cap());
        //println!("{:?}", hba.cap2());
        //println!("{}", hba.cap().num_cmd_ports());

        hba.modify_ghc(HbaMemGhcReg::IE::SET + HbaMemGhcReg::AE::SET);

        let pi = hba.pi();

        let mut disk_nr: u32 = 1;

        for i in 0..32 {
            if pi.get_bit(i) {
                let port = hba.port_mut(i);
                if port.probe(i) {
                    let addr = VirtAddr(port as *const _ as usize);

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
            self.hba = dhdr.base_address5().address_map_virt();

            self.start_hba();

            pci_data.hdr().enable_bus_mastering();
            self.enable_interrupts(pci_data);

            return true;
        }

        false
    }

    pub fn handle_interrupt(&mut self) -> bool {
        let hba = self.hba();

        let is = hba.is();
        hba.set_is(is);

        if is != 0 {
            {
                for p in 0..32 {
                    if is.get_bit(p) {
                        if let Some(p) = &self.ports[p] {
                            p.handle_interrupt();
                        }
                    }
                }
            }
            //let hba = self.hba();
            //hba.set_is(is);
        }

        return false;
    }
}
