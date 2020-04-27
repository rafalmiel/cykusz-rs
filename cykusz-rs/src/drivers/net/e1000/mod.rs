#![allow(dead_code)]

mod regs;
pub mod test;

use regs::*;

use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Once;

use crate::arch::idt::set_handler;
use crate::arch::int::{set_irq_dest, end_of_int};
use crate::arch::raw::cpuio::Port;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::raw::mm::VirtAddr;
use crate::drivers::pci::{PciDeviceHandle, PciData};
use crate::drivers::pci::{PciHeader, PciHeader0};
use crate::kernel::mm::PhysAddr;
use crate::kernel::sync::Spin;
use crate::kernel::timer::busy_sleep;

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

    fn read(&self, reg: Regs) -> u32 {
        self.read_raw(reg as u32)
    }

    fn read_raw(&self, reg: u32) -> u32 {
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

    fn write(&self, reg: Regs, val: u32) {
        self.write_raw(reg as u32, val);
    }

    fn write_raw(&self, reg: u32, val: u32) {
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

    fn flag(&self, reg: Regs, flag: u32, value: bool) {
        self.flag_raw(reg as u32, flag, value);
    }

    fn flag_raw(&self, reg: u32, flag: u32, value: bool) {
        if value {
            self.write_raw(reg, self.read_raw(reg) | flag);
        } else {
            self.write_raw(reg, self.read_raw(reg) & !flag);
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

const E1000_NUM_RX_DESCS: usize = 32;
const E1000_NUM_TX_DESCS: usize = 8;

#[allow(dead_code)]
#[repr(packed)]
#[derive(Default, Copy, Clone)]
struct RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[allow(dead_code)]
#[repr(packed)]
#[derive(Default, Copy, Clone)]
struct TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: TStatus,
    css: u8,
    special: u16,
}

struct E1000Data {
    hdr: Option<PciHeader>,
    addr: Addr,
    int_nr: u8,
    has_eeprom: bool,
    mac: [u8; 6],
    rx_ring: Vec<RxDesc>,
    tx_ring: Vec<TxDesc>,
    rx_cur: u32,
    tx_cur: u32,
}

#[allow(dead_code)]
struct E1000 {
    data: Spin<E1000Data>,
}

impl PciDeviceHandle for E1000 {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        if pci_vendor_id == 0x8086 {
            return match pci_dev_id {
                0x100E | 0x1502 => true,
                _ => false,
            };
        }

        false
    }

    fn start(&self, header: &PciHeader) -> bool {
        let mut data = self.data.lock_irq();
        data.hdr = Some(*header);

        let bar0 = data.dev_hdr().base_address0();
        let bar1 = data.dev_hdr().base_address1();

        data.addr.init(bar0, bar1);

        data.reset();

        data.read_mac();

        header.hdr().write_command(0b111);

        println!("[ E1000 ] EEPROM supported: {}", data.detect_eeprom());

        data.addr.flag(Regs::Ctrl, (ECtl::LRST | ECtl::PHY_RST).bits(), false);

        data.addr.write(Regs::FCTTV, 0);

        data.addr.flag(Regs::Ctrl, ECtl::VME.bits(), false);

        for i in 0..0x80 {
            data.addr.write_raw(0x5200 + i * 4, 0);
        }

        data.enable_interrupt();
        data.init_tx();
        data.init_rx();

        while data.addr.read(Regs::Status) & 2 != 2 {
            println!("Waiting for link up : {:X}", data.addr.read(Regs::Status));
        }

        true
    }
}

pub extern "x86-interrupt" fn e1000_handler(_frame: &mut ExceptionStackFrame) {
    let dev = device();

    dev.eoi();
    end_of_int();
}

impl E1000 {
    fn eoi(&self) {
        self.data.lock_irq().handle_irq();
    }
}

static mut BUF: *mut u8 = core::ptr::null_mut();

impl E1000Data {
    fn handle_irq(&mut self) {
        //self.addr.write(Regs::IMask, 0x1);
        let c = self.addr.read(Regs::ICause);

        if c & 0x80 == 0x80 {
            self.handle_receive();
        }
    }

    fn handle_receive(&mut self) {
        let mut desc = &mut self.rx_ring[self.rx_cur as usize];

        while desc.status & 0x1 == 0x1 {
            let buf = desc.addr;
            let len = desc.length;

            desc.status = 0;
            let old_cur = self.rx_cur;
            self.rx_cur = (self.rx_cur + 1) % E1000_NUM_RX_DESCS as u32;
            self.addr.write(Regs::RxDescTail, old_cur);

            println!("Recv packet: 0x{:x} {}", buf, len);

            let a = PhysAddr(buf as usize).to_mapped();

            for i in a..a+(len as usize) {
                unsafe {
                    print!("{:x}", i.read_volatile::<u8>())
                }
            }

            desc = &mut self.rx_ring[self.rx_cur as usize];
        }
    }

    fn reset(&self) {
        self.addr.flag(Regs::Ctrl, ECtl::RST.bits(), true);
        busy_sleep(1_000);
        while self.addr.read(Regs::Ctrl) & ECtl::RST.bits() == ECtl::RST.bits() {
            println!("Waiting for rst");
        }
    }

    fn dev_hdr(&self) -> &PciHeader0 {
        if let PciHeader::Type0(hdr) = self.hdr.as_ref().unwrap() {
            hdr
        } else {
            panic!("Invalid header")
        }
    }

    fn hdr(&self) -> &PciData {
        self.hdr.as_ref().expect("Invalid hdr").hdr()
    }

    fn link_up(&self) {
        self.addr.flag(Regs::Ctrl, ECtl::SLU.bits(), true);
    }

    fn detect_eeprom(&mut self) -> bool {
        self.addr.write(Regs::Eeprom, 1);

        for _ in 0..1000 {
            let val = self.addr.read(Regs::Eeprom);

            if val & (1 << 4) > 0 {
                self.has_eeprom = true;
                return true;
            }
        }

        false
    }

    fn eeprom_read(&self, addr: u8) -> u32 {
        let mut tmp;

        if self.has_eeprom {
            self.addr.write(Regs::Eeprom, 1 | ((addr as u32) << 8));

            loop {
                tmp = self.addr.read(Regs::Eeprom);

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

        println!(
            "[ E1000 ] MAC Address: {:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        );
    }

    fn enable_interrupt(&mut self) {
        let data = self.hdr();
        let pin = data.interrupt_pin();

        let int = crate::drivers::acpi::pci_map::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ E1000 ] Using interrupt: {}", p);

            set_irq_dest(p as u8, p as u8 + 32);
            set_handler(p as usize + 32, e1000_handler);

            self.addr.write(Regs::IMask, IntFlags::default().bits());
            self.addr.read(Regs::ICause);
        }
    }

    fn init_rx(&mut self) {
        self.rx_ring.resize_with(E1000_NUM_RX_DESCS, || {
            let mut desc = RxDesc::default();

            let buf = crate::kernel::mm::heap::allocate_align(4096, 0x1000).unwrap();

            desc.addr = VirtAddr(buf as usize).to_phys_pagewalk().unwrap().0 as u64;

            desc
        });

        let rx_addr = VirtAddr(self.rx_ring.as_ptr() as usize)
            .to_phys_pagewalk()
            .unwrap()
            .0 as u64;

        self.addr
            .write(Regs::RxDescLo, (rx_addr & 0xffff_ffff) as u32);
        self.addr.write(Regs::RxDescHi, (rx_addr >> 32) as u32);

        self.addr
            .write(Regs::RxDescLen, E1000_NUM_RX_DESCS as u32 * 16);

        self.addr.write(Regs::RxDescHead, 0);
        self.addr
            .write(Regs::RxDescTail, 31);

        let flags = RCtl::EN
            //| RCtl::SBP
            | RCtl::UPE
            | RCtl::LPE
            | RCtl::MPE
            | RCtl::LBM_NONE
            | RCtl::RDMTS_EIGHTH
            | RCtl::BAM
            | RCtl::SECRC
            | RCtl::BUF_SIZE_4096;

        self.addr.write(Regs::RCtrl, flags.bits());
    }

    fn init_tx(&mut self) {
        self.tx_ring.resize_with(E1000_NUM_TX_DESCS, || TxDesc {
            addr: 0,
            length: 0,
            cso: 0,
            cmd: 0,
            status: TStatus::DD,
            css: 0,
            special: 0,
        });

        unsafe {
            BUF = crate::kernel::mm::heap::allocate_align(4096, 0x1000).unwrap();
        }

        let tx_addr = VirtAddr(self.tx_ring.as_ptr() as usize)
            .to_phys_pagewalk()
            .unwrap()
            .0;

        self.addr
            .write(Regs::TxDescLo, (tx_addr & 0xffff_ffff) as u32);
        self.addr.write(Regs::TxDescHi, (tx_addr >> 32) as u32);

        self.addr
            .write(Regs::TxDescLen, E1000_NUM_TX_DESCS as u32 * 16);

        self.addr.write(Regs::TxDescHead, 0);
        self.addr.write(Regs::TxDescTail, 0);

        let mut flags = TCtl::default() | TCtl::EN | TCtl::PSP | TCtl::RTLC;
        flags.set_collision_threshold(15);
        flags.set_collision_distance(64);

        self.addr.write(Regs::TCtrl, 0b0110000000000111111000011111010);
        self.addr.write(Regs::Tipg, 0x00702008);
    }
}

static DEVICE: Once<Arc<E1000>> = Once::new();

fn device() -> &'static Arc<E1000> {
    DEVICE.r#try().unwrap()
}

fn init() {
    DEVICE.call_once(|| {
        Arc::new(E1000 {
            data: Spin::new(E1000Data {
                hdr: None,
                addr: Addr::new(),
                int_nr: 0,
                has_eeprom: false,
                mac: [0; 6],
                rx_ring: Vec::new(),
                tx_ring: Vec::new(),
                rx_cur: 0,
                tx_cur: 0,
            })
        })
    });

    crate::drivers::pci::register_pci_device(device().clone());
}

module_init!(init);
