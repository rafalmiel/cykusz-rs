use alloc::string::{String, ToString};
use alloc::sync::Arc;

use crate::drivers::block::ata::request::DmaRequest;
use crate::drivers::block::ide::channel::IdeChannel;
use crate::drivers::pci::{PciHeader, ProgInterface};
use crate::kernel::block::{register_blkdev, BlockDev, BlockDevice};
use crate::kernel::utils::types::CeilDiv;

pub struct IdeDrive {
    slave: bool,
    channel: Arc<IdeChannel>,
}

impl IdeDrive {
    pub fn new(slave: bool, channel: Arc<IdeChannel>) -> Arc<IdeDrive> {
        Arc::new(IdeDrive { slave, channel })
    }
}

impl BlockDev for IdeDrive {
    fn read(&self, sector: usize, dest: &mut [u8]) -> Option<usize> {
        let count = dest.len().ceil_div(512);

        let request = Arc::new(DmaRequest::new(sector, count));

        let res = self.channel.run_request(request.clone(), self.slave);

        if res.is_some() {
            request.copy_into(dest);
        }

        res
    }

    fn write(&self, sector: usize, buf: &[u8]) -> Option<usize> {
        let request = Arc::new(DmaRequest::from_bytes(sector, buf));

        self.channel.run_request(request.clone(), self.slave)
    }
}

pub struct IdeDevice {
    ide_devs: [Option<Arc<IdeDrive>>; 4],
    channels: [Option<Arc<IdeChannel>>; 2],
}

impl IdeDevice {
    pub fn new() -> IdeDevice {
        const EMPTY: Option<Arc<IdeDrive>> = None;
        const EMPTY_C: Option<Arc<IdeChannel>> = None;
        IdeDevice {
            ide_devs: [EMPTY; 4],
            channels: [EMPTY_C; 2],
        }
    }
}

impl IdeDevice {
    pub fn start(&mut self, pci_data: &PciHeader) -> bool {
        println!("[ ATA ] Starting ATA");
        let prog_if = pci_data.hdr().prog_if();

        if !prog_if.contains(ProgInterface::DMA_CAPABLE) {
            println!("[ ATA ] Error: Ata DMA not supported");
            return false;
        }

        if let PciHeader::Type0(pci) = pci_data {
            let bmid_1 = pci.base_address4() & 0xFFFF_FFFC;
            let bmid_2 = bmid_1 + 8;

            let (io1, io2) = {
                (
                    if pci.base_address0() != 0 {
                        pci.base_address0() & 0xFFFF_FFFC
                    } else {
                        0x1F0
                    },
                    if pci.base_address1() != 0 {
                        pci.base_address1() & 0xFFFF_FFFC
                    } else {
                        0x3F6
                    },
                )
            };

            let (io3, io4) = {
                (
                    if pci.base_address2() != 0 {
                        pci.base_address2() & 0xFFFF_FFFC
                    } else {
                        0x170
                    },
                    if pci.base_address3() != 0 {
                        pci.base_address3() & 0xFFFF_FFFC
                    } else {
                        0x376
                    },
                )
            };

            let c1 = IdeChannel::new(io1 as u16, io2 as u16, bmid_1 as u16, 14);
            let c2 = IdeChannel::new(io3 as u16, io4 as u16, bmid_2 as u16, 15);

            let mut idx = 0;
            for (ci, c) in [c1, c2].iter().enumerate() {
                for &s in [false, true].iter() {
                    if c.detect(s) {
                        self.ide_devs[idx] = Some(IdeDrive::new(s, c.clone()));
                        idx += 1;

                        if self.channels[ci].is_none() {
                            self.channels[ci] = Some(c.clone());
                        }
                    }
                }
            }

            if idx > 0 {
                pci_data.hdr().enable_bus_mastering();

                for c in self
                    .channels
                    .iter_mut()
                    .filter(|el| el.is_some())
                    .map(|el| el.as_mut().unwrap())
                {
                    c.init();
                }

                let mut disk_nr = 1;

                for d in self
                    .ide_devs
                    .iter()
                    .filter(|e| e.is_some())
                    .map(|e| e.as_ref().unwrap())
                {
                    if let Err(e) = register_blkdev(BlockDevice::new(
                        String::from("disk") + &disk_nr.to_string(),
                        d.clone(),
                    )) {
                        println!("[ ATA ] Failed to register blkdev {:?}", e);
                    }
                    disk_nr += 1;
                }
            }

            return true;
        }

        false
    }

    pub fn handle_interrupt(&mut self) -> bool {
        for c in self
            .channels
            .iter()
            .filter(|c| c.is_some())
            .map(|c| c.as_ref().unwrap())
        {
            c.handle_interrupt();
        }

        true
    }
}
