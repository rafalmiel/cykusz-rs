use crate::kernel::sync::Spin;
use alloc::boxed::Box;
use alloc::vec::Vec;

mod epci;
mod pci;

pub trait PciAccess: Sync {
    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64;
    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32);
}

pub trait PciDevice: Sync + Send {
    fn handles(&self, pci_dev_id: u64) -> bool;
    fn start(&self);
}

struct Pci {
    driver: Option<&'static dyn PciAccess>,
    devices: Vec<Box<dyn PciDevice>>,
}

impl Pci {
    const fn new() -> Pci {
        Pci {
            driver: None,
            devices: Vec::new(),
        }
    }

    fn check_devices(&self, dev_id: u64) {
        for dev in &self.devices {
            if dev.handles(dev_id) {
                dev.start();
            }
        }
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
                "({}, {}, {}) V: 0x{:x} D: 0x{:x} C: 0x{:x} SC: 0x{:x} p: {}, l: {} h: 0x{:x}",
                bus, device, function, vendor_id, dev_id, ccode, subclass, pin, line, hdr
            );

            self.check_devices(dev_id as u64);

            if hdr & 0b1 == 0b1 {
                let map = self.read_u32(bus, device, function, 0x18) & 0xffff;

                println!("{} -> {}", map & 0xff, map >> 8);
            }
        }
    }

    fn read_u32(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        self.driver.unwrap().read(0, bus as u16, slot as u16, func as u16, offset as u32, 32) as u32
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

static DRIVER: Spin<Pci> = Spin::new(Pci::new());

pub fn register_pci_driver(driver: &'static dyn PciAccess) {
    DRIVER.lock().driver = Some(driver);
}

pub fn register_pci_device(device: Box<dyn PciDevice>) {
    let mut driver = DRIVER.lock();

    driver.devices.push(device);
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
    let mut driver = DRIVER.lock();

    driver.init();
}

pub fn read(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
    DRIVER.lock().driver.unwrap().read(seg, bus, dev, fun, reg, width)
}

pub fn write(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
    DRIVER
        .lock()
        .driver
        .unwrap()
        .write(seg, bus, dev, fun, reg, val, width);
}

platform_init!(init);
