use crate::arch::raw::cpuio::Port;
use crate::kernel::sync::Spin;

struct Pci {
    addr: Port<u32>,
    data: Port<u32>,
}

impl Pci {
    fn new() -> Pci {
        unsafe {
            Pci {
                addr: Port::new(0xCF8),
                data: Port::new(0xCFC),
            }
        }
    }

    fn read_u32(&mut self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let addr = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((offset as u32) & 0xfc)
            | 0x80000000u32;

        self.addr.write(addr);

        let res = self.data.read();

        return res;
    }

    fn check(&mut self, bus: u8, device: u8, function: u8) {
        let vid_did = self.read_u32(bus, device, function, 0);

        if vid_did != 0xffffffff {
            let vendor_id = vid_did & 0xffff;
            let dev_id = vid_did >> 16;

            let class = self.read_u32(bus, device, function, 8);

            let ccode = class >> 24;
            let subclass = (class >> 16) & 0xff;

            let int = self.read_u32(bus, device, function, 0x3c);

            let line = int & 0xff;
            let pin = (int >> 8) & 0xff;

            println!(
                "({}, {}, {})V: 0x{:x} D: 0x{:x} C: 0x{:x} SC: 0x{:x} p: {}, l: {}",
                bus, device, function, vendor_id, dev_id, ccode, subclass, pin, line
            );
        }
    }

    pub fn init(&mut self) {
        for bus in 0..=255 {
            for device in 0..32 {
                self.check(bus, device, 0);
                let header = (self.read_u32(bus, device, 0, 0xc) >> 16) & 0xff;

                if header & 0x80 > 0 {
                    for f in 0..8 {
                        self.check(bus, device, f);
                    }
                }
            }
        }
    }
}

pub fn pci_init() {
    let mut pci = PCI.lock();

    *pci = Some(Pci::new());

    pci.as_mut().unwrap().init();
}

static PCI: Spin<Option<Pci>> = Spin::new(None);

pub fn read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    if let Some(ref mut pci) = *PCI.lock() {
        pci.read_u32(bus, slot, func, offset)
    } else {
        panic!("PCI read failed");
    }
}

platform_init!(pci_init);
