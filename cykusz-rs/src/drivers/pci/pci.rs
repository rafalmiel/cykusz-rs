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
