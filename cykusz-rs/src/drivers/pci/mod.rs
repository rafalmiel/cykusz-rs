use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::kernel::sync::Spin;

mod epci;
mod pci;

pub trait PciAccess: Sync {
    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64;
    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32);
}

pub trait PciDeviceHandle: Sync + Send {
    fn handles(&self, pci_dev_id: u64) -> bool;
    fn start(&self, pci_data: &PciData);
}

#[derive(Copy, Clone)]
struct PciHeader0 {}
#[derive(Copy, Clone)]
struct PciHeader1 {}
#[derive(Copy, Clone)]
struct PciHeader2 {}

#[derive(Copy, Clone)]
enum PciHeader {
    Unknown,
    Type0(PciHeader0),
    Type1(PciHeader1),
    Type2(PciHeader2),
}

#[derive(Copy, Clone)]
pub struct PciData {
    pub seg: u16,
    pub bus: u16,
    pub dev: u16,
    pub fun: u16,
    header: PciHeader,
}

#[allow(dead_code)]
impl PciData {
    pub fn new() -> PciData {
        PciData {
            seg: 0,
            bus: 0,
            dev: 0,
            fun: 0,
            header: PciHeader::Unknown,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.header_type() != 0xff
    }

    pub fn init(&mut self, seg: u16, bus: u16, dev: u16, fun: u16) -> bool {
        self.seg = seg;
        self.bus = bus;
        self.dev = dev;
        self.fun = fun;

        if !self.is_valid() {
            return false;
        }

        match self.header_type() & 0b01111111 {
            0x0 => self.header = PciHeader::Type0(PciHeader0 {}),
            0x1 => self.header = PciHeader::Type1(PciHeader1 {}),
            0x2 => self.header = PciHeader::Type2(PciHeader2 {}),
            _ => {
                panic!("Invalid PCI Header");
            }
        }

        return true;
    }

    pub fn debug(&self) {
        println!(
            "({}, {}, {}) V: 0x{:x} D: 0x{:x} C: 0x{:x} SC: 0x{:x} p: {}, l: {} h: 0x{:x}",
            self.bus,
            self.dev,
            self.fun,
            self.vendor_id(),
            self.device_id(),
            self.class(),
            self.subclass(),
            self.interrupt_pin(),
            self.interrupt_line(),
            self.header_type()
        );
    }

    fn read(&self, offset: u32, width: u32) -> u64 {
        read(self.seg, self.bus, self.dev, self.fun, offset, width)
    }

    pub fn vendor_id(&self) -> u16 {
        self.read(0x00, 16) as u16
    }

    pub fn device_id(&self) -> u16 {
        self.read(0x02, 16) as u16
    }

    pub fn revision_id(&self) -> u8 {
        self.read(0x08, 8) as u8
    }

    pub fn prog_if(&self) -> u8 {
        self.read(0x09, 8) as u8
    }

    pub fn subclass(&self) -> u8 {
        self.read(0xA, 8) as u8
    }

    pub fn class(&self) -> u8 {
        self.read(0xB, 8) as u8
    }

    pub fn cacheline_size(&self) -> u8 {
        self.read(0xC, 8) as u8
    }

    pub fn latency_timer(&self) -> u8 {
        self.read(0xD, 8) as u8
    }

    pub fn header_type(&self) -> u8 {
        self.read(0xE, 8) as u8
    }

    pub fn bist(&self) -> u8 {
        self.read(0xF, 8) as u8
    }

    pub fn interrupt_pin(&self) -> u8 {
        self.read(0x3D, 8) as u8
    }

    pub fn interrupt_line(&self) -> u8 {
        self.read(0x3C, 8) as u8
    }
}

struct PciDevice {
    handle: Box<dyn PciDeviceHandle>,
    found: bool,
    data: PciData,
}

struct Pci {
    devices: Vec<PciDevice>,
}

impl Pci {
    const fn new() -> Pci {
        Pci {
            devices: Vec::new(),
        }
    }

    fn check_devices(&mut self, pci_data: &PciData) {
        let dev_id = pci_data.device_id();

        for dev in &mut self.devices {
            if dev.handle.handles(dev_id as u64) {
                dev.found = true;

                dev.data = *pci_data;

                dev.handle.start(&dev.data);
            }
        }
    }

    fn check(&mut self, bus: u8, device: u8, function: u8) {
        let mut pci_data = PciData::new();
        let succeeded = pci_data.init(0, bus as u16, device as u16, function as u16);

        if succeeded {
            pci_data.debug();

            self.check_devices(&pci_data);
        }
    }

    fn read_u32(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        DRIVER
            .lock()
            .unwrap()
            .read(0, bus as u16, slot as u16, func as u16, offset as u32, 32) as u32
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

static DRIVER: Spin<Option<&'static dyn PciAccess>> = Spin::new(None);
static PCI: Spin<Pci> = Spin::new(Pci::new());

pub fn register_pci_driver(driver: &'static dyn PciAccess) {
    *DRIVER.lock() = Some(driver);
}

pub fn register_pci_device(device: Box<dyn PciDeviceHandle>) {
    let mut driver = PCI.lock();

    driver.devices.push(PciDevice {
        handle: device,
        found: false,
        data: PciData::new(),
    });
}

pub fn init() {
    if !epci::init() {
        pci::init();
        println!("[ OK ] PCI Initialized");
    } else {
        println!("[ OK ] Express PCI Initialized");
    }
}

pub fn enumerate_pci() {
    let mut driver = PCI.lock();

    driver.init();
}

pub fn read(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
    DRIVER.lock().unwrap().read(seg, bus, dev, fun, reg, width)
}

pub fn write(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
    DRIVER
        .lock()
        .unwrap()
        .write(seg, bus, dev, fun, reg, val, width);
}

platform_init!(init);
