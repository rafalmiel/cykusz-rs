use alloc::vec::Vec;

use crate::arch::idt::set_handler;
use crate::arch::int::{end_of_int, set_active_high, set_irq_dest};
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::raw::mm::{PhysAddr, VirtAddr};
use crate::drivers::net::e1000::addr::Addr;
use crate::drivers::net::e1000::regs::Regs;
use crate::drivers::pci::{PciData, PciHeader, PciHeader0};
use crate::kernel::mm::heap::allocate_align;
use crate::kernel::timer::busy_sleep;

use super::regs::*;
use super::*;

pub const E1000_NUM_RX_DESCS: usize = 32;
pub const E1000_NUM_TX_DESCS: usize = 8;

#[allow(dead_code)]
#[repr(packed)]
#[derive(Default, Copy, Clone)]
pub struct RxDesc {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[allow(dead_code)]
#[repr(packed)]
#[derive(Default, Copy, Clone)]
pub struct TxDesc {
    pub addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: TStatus,
    pub css: u8,
    pub special: u16,
}

pub struct E1000Data {
    pub hdr: Option<PciHeader>,
    pub addr: Addr,
    pub int_nr: u8,
    pub has_eeprom: bool,
    pub mac: [u8; 6],
    pub rx_ring: Vec<RxDesc>,
    pub tx_ring: Vec<TxDesc>,
    pub rx_cur: u32,
    pub tx_cur: u32,
    pub ring_buf: VirtAddr,
}

pub extern "x86-interrupt" fn e1000_handler(_frame: &mut ExceptionStackFrame) {
    let dev = device();

    dev.handle_irq();

    end_of_int();
}

pub static mut BUF: *mut u8 = core::ptr::null_mut();

impl E1000Data {
    pub fn init(&mut self, hdr: &PciHeader) {
        self.hdr = Some(*hdr);

        let bar0 = self.dev_hdr().base_address0();
        let bar1 = self.dev_hdr().base_address1();

        self.addr.init(bar0, bar1);

        let ring_buf = allocate_align(0x1000, 0x1000).unwrap();
        unsafe {
            ring_buf.write_bytes(0, 0x1000);
        }
        self.ring_buf = VirtAddr(ring_buf as usize);
    }

    pub fn handle_irq(&mut self) {
        //self.addr.write(Regs::IMask, 0x1);
        let c = self.addr.read(Regs::ICause);

        if c & 0x80 == 0x80 {
            self.handle_receive();
        }
    }

    pub fn handle_receive(&mut self) {
        let mut desc = &mut self.rx_ring[self.rx_cur as usize];

        while desc.status & 0x1 == 0x1 {
            let buf = desc.addr;
            let len = desc.length;

            desc.status = 0;
            let old_cur = self.rx_cur;
            self.rx_cur = (self.rx_cur + 1) % E1000_NUM_RX_DESCS as u32;
            self.addr.write(Regs::RxDescTail, old_cur);

            //print!(".");

            println!("Recv packet: 0x{:x} {}", buf, len);

            let a = PhysAddr(buf as usize).to_mapped();

            for i in a..a + (len as usize) {
                unsafe { print!("{:x}", i.read_volatile::<u8>()) }
            }

            desc = &mut self.rx_ring[self.rx_cur as usize];
        }
    }

    pub fn reset(&self) {
        self.addr.flag(Regs::Ctrl, ECtl::RST.bits(), true);

        busy_sleep(1_000);

        while self.addr.read(Regs::Ctrl) & ECtl::RST.bits() == ECtl::RST.bits() {
            println!("Waiting for rst");
        }

        self.addr
            .flag(Regs::Ctrl, (ECtl::LRST | ECtl::PHY_RST).bits(), false);

        self.addr.flag(Regs::Ctrl, ECtl::VME.bits(), false);
    }

    pub fn wait_link_up(&self) {
        while self.addr.read(Regs::Status) & 2 != 2 {
            println!("Waiting for link up : {:X}", self.addr.read(Regs::Status));
        }
    }

    pub fn clear_filters(&self) {
        for i in 0..0x80 {
            self.addr.write_raw(0x5200 + i * 4, 0);
        }
    }

    pub fn dev_hdr(&self) -> &PciHeader0 {
        if let PciHeader::Type0(hdr) = self.hdr.as_ref().unwrap() {
            hdr
        } else {
            panic!("Invalid header")
        }
    }

    pub fn hdr(&self) -> &PciData {
        self.hdr.as_ref().expect("Invalid hdr").hdr()
    }

    pub fn link_up(&self) {
        self.addr.flag(Regs::Ctrl, ECtl::SLU.bits(), true);
    }

    pub fn detect_eeprom(&mut self) {
        self.addr.write(Regs::Eeprom, 1);

        for _ in 0..1000 {
            let val = self.addr.read(Regs::Eeprom);

            if val & (1 << 4) > 0 {
                self.has_eeprom = true;
                break;
            }
        }

        println!("[ E1000 ] EEPROM supported: {}", self.has_eeprom);
    }

    pub fn eeprom_read(&self, addr: u8) -> u32 {
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

    pub fn read_mac(&mut self) {
        if self.has_eeprom {
            for i in 0..3 {
                let t = self.eeprom_read(i) as u16;
                self.mac[i as usize * 2] = (t & 0xff) as u8;
                self.mac[i as usize * 2 + 1] = (t >> 8) as u8;
            }
        } else {
            let base = PhysAddr(self.addr.base() as usize).to_mapped() + 0x5400;

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

    pub fn enable_interrupt(&mut self) {
        let data = self.hdr();
        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ E1000 ] Using interrupt: {}", p);

            set_irq_dest(p as u8, p as u8 + 32);
            set_active_high(p as u8, true);
            set_handler(p as usize + 32, e1000_handler);

            self.addr.write(Regs::IMask, IntFlags::default().bits());
            self.addr.read(Regs::ICause);
        }
    }

    pub fn init_rx(&mut self) {
        self.rx_ring = unsafe {
            Vec::from_raw_parts(
                (self.ring_buf + 512).0 as *mut RxDesc,
                E1000_NUM_RX_DESCS,
                E1000_NUM_RX_DESCS,
            )
        };
        for r in self.rx_ring.iter_mut() {
            let buf = crate::kernel::mm::heap::allocate_align(4096, 0x1000).unwrap();

            r.addr = VirtAddr(buf as usize).to_phys_pagewalk().unwrap().0 as u64;
        }

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
        self.addr.write(Regs::RxDescTail, 31);

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

    pub fn init_tx(&mut self) {
        self.tx_ring = unsafe {
            Vec::from_raw_parts(
                self.ring_buf.0 as *mut TxDesc,
                E1000_NUM_TX_DESCS,
                E1000_NUM_TX_DESCS,
            )
        };
        for t in self.tx_ring.iter_mut() {
            t.status = TStatus::DD;
        }

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

        self.addr.write(Regs::TCtrl, flags.bits());
        self.addr.write(Regs::Tipg, 0x00702008);
    }
}
