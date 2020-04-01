use crate::kernel::sync::Spin;

mod epci;
mod pci;

pub trait PciAccess: Sync {
    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64;
    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32);
}

static DRIVER: Spin<Option<&'static dyn PciAccess>> = Spin::new(None);

pub fn register_pci_driver(driver: &'static dyn PciAccess) {
    *DRIVER.lock() = Some(driver);
}

pub fn init() {
    println!("Platform init pci:");
    if !epci::init() {
        println!("Platform init pci 2:");
        pci::init();
    }
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
