use crate::arch::raw::cpuio::Port;
use crate::drivers::pci::PciAccess;
use crate::kernel::sync::Spin;

struct Ports {
    addr: Port<u32>,
    data: Port<u32>,
}

struct Pci {
    ports: Spin<Ports>,
}

impl Pci {
    fn new() -> Pci {
        unsafe {
            Pci {
                ports: Spin::new(Ports {
                    addr: Port::new(0xCF8),
                    data: Port::new(0xCFC),
                }),
            }
        }
    }

    fn read_u32(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let addr = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((offset as u32) & 0xfc)
            | 0x80000000u32;

        let mut p = self.ports.lock();

        p.addr.write(addr);

        let res = p.data.read();

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

            let hdr = (self.read_u32(bus, device, function, 0xc) >> 16) & 0xff;

            let line = int & 0xff;
            let pin = (int >> 8) & 0xff;

            println!(
                "({}, {}, {})V: 0x{:x} D: 0x{:x} C: 0x{:x} SC: 0x{:x} p: {}, l: {} h: 0x{:x}",
                bus, device, function, vendor_id, dev_id, ccode, subclass, pin, line, hdr
            );

            if hdr & 0b1 == 0b1 {
                let map = self.read_u32(bus, device, function, 0x18) & 0xffff;

                println!("{} -> {}", map & 0xff, map >> 8);
            }
        }
    }

    pub fn init(&mut self) {
        for bus in 0..=255 {
            for device in 0..32 {
                self.check(bus, device, 0);
                let header = (self.read_u32(bus, device, 0, 0xc) >> 16) & 0xff;

                if header & 0x80 > 0 {
                    for f in 1..8 {
                        self.check(bus, device, f);
                    }
                }
            }
        }
    }
}

static DRIVER: Driver = Driver {};

struct Driver {}

impl PciAccess for Driver {
    fn read(&self, _seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
        let mut val = PCI
            .lock()
            .as_ref()
            .unwrap()
            .read_u32(bus as u8, dev as u8, fun as u8, reg as u8) as u64;

        match width {
            8 => {
                let offset = (reg & 0b11) * 8;

                val = (val >> offset) as u8 as u64
            }
            16 => {
                let offset = (reg & 0b11) * 8;

                val = (val >> offset) as u16 as u64
            }
            32 => {}
            _ => {
                panic!("Unsupported width");
            }
        }

        val
    }

    fn write(&self, _seg: u16, _bus: u16, _dev: u16, _fun: u16, _reg: u32, _val: u64, _width: u32) {
    }
}

pub fn init() {
    pci_init();

    super::register_pci_driver(&DRIVER);
}

pub fn pci_init() {
    let mut pci = PCI.lock();

    *pci = Some(Pci::new());

    pci.as_mut().unwrap().init();
}

static PCI: Spin<Option<Pci>> = Spin::new(None);

pub fn read(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
    DRIVER.read(seg, bus, dev, fun, reg, width)
}

#[allow(dead_code)]
pub fn read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    if let Some(ref mut pci) = *PCI.lock() {
        pci.read_u32(bus, slot, func, offset)
    } else {
        panic!("PCI read failed");
    }
}
