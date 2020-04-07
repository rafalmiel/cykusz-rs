use crate::drivers::pci::PciAccess;
use crate::kernel::mm::{MappedAddr, PhysAddr};
use crate::kernel::sync::Spin;

struct Mcfg(Option<&'static acpica::acpi_table_mcfg>);

impl Mcfg {
    const fn new() -> Mcfg {
        Mcfg { 0: None }
    }

    fn init(&mut self, hdr: &'static acpica::acpi_table_mcfg) {
        self.0 = Some(hdr);
    }

    fn find_addr(&self, segment: u16, bus: u16, device: u16, function: u16) -> Option<MappedAddr> {
        let mut cfg = unsafe {
            (self.0? as *const acpica::acpi_table_mcfg).offset(1)
                as *const acpica::acpi_mcfg_allocation
        };

        let mut len = (self.0?.Header.Length as usize
            - core::mem::size_of::<acpica::acpi_table_mcfg>()) as isize;

        while len > 0 {
            let c = unsafe { &*cfg };

            if c.PciSegment == segment
                && (c.StartBusNumber as u16 <= bus && bus <= c.EndBusNumber as u16)
            {
                let addr = PhysAddr(c.Address as usize).to_mapped();

                return Some(
                    addr + ((bus as usize - c.StartBusNumber as usize) << 20)
                        | ((device as usize) << 15)
                        | ((function as usize) << 12),
                );
            }

            len -= core::mem::size_of::<acpica::acpi_mcfg_allocation>() as isize;

            cfg = unsafe { cfg.offset(1) };
        }

        return None;
    }

    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
        if let Some(addr) = self.find_addr(seg, bus, dev, fun) {
            unsafe {
                match width {
                    8 => (addr + reg as usize).store_volatile(val as u8),
                    16 => (addr + reg as usize).store_volatile(val as u16),
                    32 => (addr + reg as usize).store_volatile(val as u32),
                    64 => (addr + reg as usize).store_volatile(val as u64),
                    _ => panic!("Invalid Width"),
                }
            }
        } else {
            panic!("Failed pci write")
        }
    }

    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
        if let Some(addr) = self.find_addr(seg, bus, dev, fun) {
            unsafe {
                return match width {
                    8 => (addr + reg as usize).read_volatile::<u8>() as u64,
                    16 => (addr + reg as usize).read_volatile::<u16>() as u64,
                    32 => (addr + reg as usize).read_volatile::<u32>() as u64,
                    64 => (addr + reg as usize).read_volatile::<u64>() as u64,
                    _ => panic!("Invalid Width"),
                };
            }
        }

        crate::drivers::pci::pci::read(seg, bus, dev, fun, reg, width)
    }
}

static EXPCI: Spin<Mcfg> = Spin::new(Mcfg::new());

static DRIVER: Driver = Driver {};

struct Driver {}

impl PciAccess for Driver {
    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
        EXPCI.lock().read(seg, bus, dev, fun, reg, width)
    }

    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
        EXPCI.lock().write(seg, bus, dev, fun, reg, val, width);
    }
}

pub fn init() -> bool {
    if let Some(m) = crate::arch::acpi::ACPI.lock().get_mcfg_entry() {
        init_pci(m);

        super::register_pci_driver(&DRIVER);

        return true;
    }

    return false;
}

pub fn init_pci(hdr: &'static acpica::acpi_table_mcfg) {
    let mut e = EXPCI.lock();

    e.init(hdr);

    super::pci::pci_init();
}

#[allow(dead_code)]
pub fn write(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
    EXPCI.lock().write(seg, bus, dev, fun, reg, val, width);
}

#[allow(dead_code)]
pub fn read(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
    EXPCI.lock().read(seg, bus, dev, fun, reg, width)
}
