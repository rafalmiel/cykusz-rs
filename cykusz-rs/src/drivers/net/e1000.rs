use alloc::boxed::Box;

use crate::arch::raw::cpuio::Port;
use crate::arch::raw::mm::PhysAddr;
use crate::drivers::pci::PciDeviceHandle;
use crate::drivers::pci::{PciHeader, PciHeader0};

struct Addr {
    mmio: bool,
    base: u64,
}

impl Addr {
    pub fn new() -> Addr {
        Addr {
            mmio: false,
            base: 0,
        }
    }

    fn read(&self, reg: u16) -> u32 {
        if self.mmio {
            unsafe {
                return PhysAddr(self.base as usize + reg as usize)
                    .to_mapped()
                    .read_volatile::<u32>();
            }
        } else {
            unsafe {
                Port::<u32>::new(self.base as u16).write(reg as u32);
                return Port::<u32>::new(self.base as u16 + 0x4).read();
            }
        }
    }

    fn write(&self, reg: u16, val: u32) {
        if self.mmio {
            unsafe {
                return PhysAddr(self.base as usize + reg as usize)
                    .to_mapped()
                    .store_volatile(val);
            }
        } else {
            unsafe {
                Port::<u32>::new(self.base as u16).write(reg as u32);
                Port::<u32>::new(self.base as u16 + 0x4).write(val);
            }
        }
    }

    fn init(&mut self, bar0: u32, bar1: u32) {
        self.mmio = (bar0 & 0b1) == 0;

        if self.mmio {
            if (bar0 >> 1) & 0b11 == 2 {
                self.base = (bar0 as u64 & 0xffff_fff0) + ((bar1 as u64) << 32);
            } else {
                self.base = bar0 as u64 & 0xffff_fff0;
            }
        } else {
            self.base = bar0 as u64 & 0xffff_fffc;
        }
    }
}

struct E1000 {
    hdr: Option<PciHeader0>,
    addr: Addr,
    has_eeprom: bool,
    mac: [u8; 6],
}

impl PciDeviceHandle for E1000 {
    fn handles(&self, pci_dev_id: u64) -> bool {
        return match pci_dev_id {
            0x100E | 0x1502 => true,
            _ => false,
        };
    }

    fn start(&mut self, header: &PciHeader) -> bool {
        match header {
            PciHeader::Type0(hdr) => self.hdr = Some(*hdr),
            _ => {
                return false;
            }
        }

        let bar0 = self.hdr.unwrap().base_address0();
        let bar1 = self.hdr.unwrap().base_address1();

        self.addr.init(bar0, bar1);
        let has_eeprom = self.detect_eeprom();

        println!("[ E1000 ] EEPROM supported: {}", has_eeprom);

        self.read_mac();

        println!(
            "[ E1000 ] MAC Address: {:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        );

        true
    }
}

impl E1000 {
    fn detect_eeprom(&mut self) -> bool {
        self.addr.write(0x14, 1);

        for _ in 0..1000 {
            let val = self.addr.read(0x14);

            if val & 0x10 > 0 {
                self.has_eeprom = true;
                return true;
            }
        }

        false
    }

    fn eeprom_read(&self, addr: u8) -> u32 {
        let mut tmp;

        if self.has_eeprom {
            self.addr.write(0x14, 1 | ((addr as u32) << 8));

            loop {
                tmp = self.addr.read(0x14);

                if tmp & (1 << 4) > 0 {
                    break;
                }
            }
        } else {
            panic!("EEPROM Does not exists");
        }

        return (tmp >> 16) & 0xffff;
    }

    fn read_mac(&mut self) {
        if self.has_eeprom {
            for i in 0..3 {
                let t = self.eeprom_read(i) as u16;
                self.mac[i as usize * 2] = (t & 0xff) as u8;
                self.mac[i as usize * 2 + 1] = (t >> 8) as u8;
            }
        } else {
            let base = PhysAddr(self.addr.base as usize).to_mapped() + 0x5400;

            unsafe {
                if base.read_volatile::<u32>() != 0 {
                    for i in 0..6 {
                        self.mac[i] = (base + i).read_volatile::<u8>();
                    }
                }
            }
        }
    }
}

fn init() {
    crate::drivers::pci::register_pci_device(Box::new(E1000 {
        hdr: None,
        addr: Addr::new(),
        has_eeprom: false,
        mac: [0; 6],
    }));
}

module_init!(init);
