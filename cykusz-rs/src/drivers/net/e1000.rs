#![allow(dead_code)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Once;

use crate::arch::idt::set_handler;
use crate::arch::int::{mask_int, set_irq_dest, end_of_int};
use crate::arch::raw::cpuio::Port;
use crate::arch::raw::idt::ExceptionStackFrame;
use crate::arch::raw::mm::VirtAddr;
use crate::drivers::pci::PciDeviceHandle;
use crate::drivers::pci::{PciHeader, PciHeader0};
use crate::kernel::mm::PhysAddr;
use crate::kernel::sync::Spin;
use crate::kernel::timer::busy_sleep;
use crate::kernel::sched::current_task;

#[repr(u16)]
enum Regs {
    Ctrl = 0x0000,
    Status = 0x0008,
    Eeprom = 0x0014,
    CtrlExt = 0x0018,
    PHY = 0x0020,
    FCAL = 0x0028,
    FCAH = 0x02C,
    FCT = 0x0030,
    FCTTV = 0x0170,
    ICause = 0x00C0,
    ICauseSet = 0x00C8,
    IMask = 0x00D0,
    IMaskClr = 0x00D8,
    RCtrl = 0x0100,
    RxDescLo = 0x2800,
    RxDescHi = 0x2804,
    RxDescLen = 0x2808,
    RxDescHead = 0x2810,
    RxDescTail = 0x2818,

    TCtrl = 0x0400,
    TxDescLo = 0x3800,
    TxDescHi = 0x3804,
    TxDescLen = 0x3808,
    TxDescHead = 0x3810,
    TxDescTail = 0x3818,

    Rdtr = 0x2820,    // RX Delay Timer Register
    RxDCtrl = 0x3828, // RX Descriptor Control
    Radv = 0x282C,    // RX Int. Absolute Delay Timer
    Rsrpd = 0x2C00,   // RX Small Packet Detect Interrupt

    Tipg = 0x0410, // Transmit Inter Packet Gap

    RcCount = 0x2430,
}

bitflags! {
    pub struct ECtl: u32 {
        const LRST          = (1 << 3);
        const ASDE          = (1 << 5);
        const SLU           = (1 << 6);     // Set Link Up
        const ILOS          = (1 << 7);
        const RST           = (1 << 26);
        const VME           = (1 << 30);
        const PHY_RST       = (1 << 31);
    }
}

bitflags! {
    pub struct RCtl: u32 {
        const EN            = (1 << 1);     // Receiver Enable
        const SBP           = (1 << 2);     // Store Bad Packets
        const UPE           = (1 << 3);     // Unicast Promiscuous Enabled
        const MPE           = (1 << 4);     // Multicast Promiscuous Enabled
        const LPE           = (1 << 5);     // Long Packet Reception Enable
        const LBM_NONE      = (0 << 6);     // No Loopback
        const LBM_PHY       = (3 << 6);     // PHY or external SerDesc loopback
        const RDMTS_HALF    = (0 << 8);     // Free Buffer Threshold is 1/2 of RDLEN
        const RDMTS_QUARTER = (1 << 8);     // Free Buffer Threshold is 1/4 of RDLEN
        const RDMTS_EIGHTH  = (2 << 8);     // Free Buffer Threshold is 1/8 of RDLEN
        const MO_36         = (0 << 12);    // Multicast Offset - bits 47:36
        const MO_35         = (1 << 12);    // Multicast Offset - bits 46:35
        const MO_34         = (2 << 12);    // Multicast Offset - bits 45:34
        const MO_32         = (3 << 12);    // Multicast Offset - bits 43:32
        const BAM           = (1 << 15);    // Broadcast Accept Mode
        const VFE           = (1 << 18);    // VLAN Filter Enable
        const CFIEN         = (1 << 19);    // Canonical Form Indicator Enable
        const CFI           = (1 << 20);    // Canonical Form Indicator Bit Value
        const DPF           = (1 << 22);    // Discard Pause Frames
        const PMCF          = (1 << 23);    // Pass MAC Control Frames
        const SECRC         = (1 << 26);    // Strip Ethernet CRC

        const BUF_SIZE_256  = (3 << 16);
        const BUF_SIZE_512  = (2 << 16);
        const BUF_SIZE_1024 = (1 << 16);
        const BUF_SIZE_2048 = (0 << 16);
        const BUF_SIZE_4096 = ((3 << 16) | (1 << 25));
        const BUF_SIZE_8192 = ((2 << 16) | (1 << 25));
        const BUF_SIZE_16384= ((1 << 16) | (1 << 25));
    }
}

bitflags! {
    pub struct TCmd: u8 {
        const EOP           = (1 << 0);     // End of Packet
        const IFCS          = (1 << 1);     // Insert FCS
        const IC            = (1 << 2);     // Insert Checksum
        const RS            = (1 << 3);     // Report Status
        const RPS           = (1 << 4);     // Report Packet Sent
        const VLE           = (1 << 6);     // VLAN Packet Enable
        const IDE           = (1 << 7);     // Interrupt Delay Enable
    }
}

bitflags! {
    pub struct TCtl : u32 {
        const EN            = (1 << 1);     // Transmit Enable
        const PSP           = (1 << 3);     // Pad Short Packets
        const SWXOFF        = (1 << 22);    // Software XOFF Transmission
        const RTLC          = (1 << 24);    // Re-transmit on Late Collision
    }
}

bitflags! {
    pub struct TStatus: u8 {
        const DD            = (1 << 0);     // Descriptor Done
        const EC            = (1 << 1);     // Excess Collisions
        const LC            = (1 << 2);     // Late Collision
        const TU            = (1 << 3);     // Transmit Underrun
    }
}

impl Default for TStatus {
    fn default() -> Self {
        TStatus { bits: 0 }
    }
}

impl TCtl {
    fn set_collision_threshold(&mut self, val: u8) {
        self.bits |= (val as u32) << 4;
    }

    fn set_collision_distance(&mut self, val: u8) {
        self.bits |= (val as u32) << 12;
    }

    fn set_read_request_threshold(&mut self, val: u8) {
        self.bits |= (val as u32) << 29;
    }
}

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


pub fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..10000000000u64 {
        unsafe {
            let _ = a.read_volatile();
        }
    }
}

pub fn test() {
    let mut data = device().data.lock_irq();
    println!("RcCount: {}", data.addr.read(Regs::RcCount));
    println!("RxHead: {}", data.addr.read(Regs::RxDescHead));
    println!("RxTail: {}", data.addr.read(Regs::RxDescTail));
    println!("PCI Status 0x{:x}", data.hdr().hdr().status());
    println!("PCI Command 0x{:x}", data.hdr().hdr().command());
    //println!("RxCause: 0x{:x}", data.addr.read(Regs::ICause));
    println!("IAS: 0x{:x}", data.addr.read_raw(0x4100));
    //println!("IMask: 0x{:x}", data.addr.read(Regs::IMask));
    let mut d = 0u32;
    d |= (1u32 << 29);
    d |= (2u32 << 26);
    d |= (1u32 << 21);

    data.addr.write(Regs::PHY, d);

    data.send_test();
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

        data.read_mac();

        println!(
            "[ E1000 ] MAC Address: {:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            data.mac[0], data.mac[1], data.mac[2], data.mac[3], data.mac[4], data.mac[5]
        );

        header.hdr().write_command(0b111);

        println!("Status: {:x}", data.addr.read(Regs::Status));

        let has_eeprom = data.detect_eeprom();

        println!("[ E1000 ] EEPROM supported: {}", has_eeprom);

        data.addr.flag(Regs::Ctrl, ECtl::RST.bits, true);

        busy_sleep(10000);

        while data.addr.read(Regs::Ctrl) & ECtl::RST.bits == ECtl::RST.bits {
            println!("Waiting for rst");
        }

        data.addr.flag(Regs::Ctrl, (ECtl::SLU | ECtl::ASDE).bits, true);
        data.addr.flag(Regs::Ctrl, (ECtl::LRST | ECtl::PHY_RST | ECtl::ILOS).bits, false);

        //data.addr.write(Regs::FCAL, 0);
        //data.addr.write(Regs::FCAH, 0);
        //data.addr.write(Regs::FCT, 0);
        //data.addr.write(Regs::FCTTV, 0);

        data.addr.flag(Regs::Ctrl, ECtl::VME.bits, false);

        for i in 0..0x80 {
            data.addr.write_raw(0x5200 + i*4, 0);
        }

        data.enable_interrupt();
        data.init_tx();
        data.init_rx();

        while data.addr.read(Regs::Status) & 2 != 2 {
            println!("Waiting for link up : {:X}", data.addr.read(Regs::Status));
        }

        println!("Status: {:x}", data.addr.read(Regs::Status));
        println!("Ctrl: {:x}", data.addr.read(Regs::Ctrl));

        println!("PHY: 0x{:x}", data.addr.read(Regs::PHY));
        //let mut d = 0u32;
        //d |= (1u32 << 29);
        //d |= (2u32 << 26);
        //d |= (1u32 << 21);


        //data.addr.write(Regs::PHY, d);
        //println!("PHY: 0x{:x}", data.addr.read(Regs::PHY));
        println!("IAM: 0x{:x}", data.addr.read_raw(0x000E0));


        //for _ in 1..10 {
        //    data.addr.write(Regs::ICauseSet, 0xffff);
        //    dummy_work();
        //}

        //loop {
        //    println!("Rc Count: {}", data.addr.read(Regs::RcCount));
        //}

        println!("PCI Status 0x{:x}", data.hdr().hdr().status());
        println!("PCI Command 0x{:x}", data.hdr().hdr().command());

        true

    }
    /*fn start(&self, header: &PciHeader) -> bool {
        let mut data = self.data.lock_irq();
        println!("STS: 0x{:x}", header.hdr().status());

        data.hdr = Some(*header);

        let bar0 = data.dev_hdr().base_address0();
        let bar1 = data.dev_hdr().base_address1();

        data.addr.init(bar0, bar1);

        let has_eeprom = data.detect_eeprom();

        println!("[ E1000 ] EEPROM supported: {}", has_eeprom);

        data.read_mac();

        let mut ctr = data.addr.read(Regs::Ctrl);
        ctr |= ECtl::RST.bits;
        data.addr.write(Regs::Ctrl, ctr);

        busy_sleep(10000);

        while data.addr.read(Regs::Ctrl) & ECtl::RST.bits == ECtl::RST.bits {
            println!("Waiting");
        }

        //let mut ctr = data.addr.read(Regs::Ctrl);
        //ctr |= ECtl::LRST.bits;
        //data.addr.write(Regs::Ctrl, ctr);

        //busy_sleep(10000);

        //while data.addr.read(Regs::Ctrl) & ECtl::LRST.bits == ECtl::LRST.bits {
        //    println!("Waiting");
        //}

        println!("Reset complete");

        let mut ctr = data.addr.read(Regs::Ctrl);
        ctr |= (ECtl::SLU | ECtl::ASDE).bits;
        data.addr.write(Regs::Ctrl, ctr);

        busy_sleep(10000);

        for i in 0..0x80 {
            data.addr.write_raw(0x5200 + i*4, 0);
        }

        println!(
            "[ E1000 ] MAC Address: {:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            data.mac[0], data.mac[1], data.mac[2], data.mac[3], data.mac[4], data.mac[5]
        );

        data.addr.write(Regs::Ctrl, ECtl::SLU.bits as u32);

        data.enable_interrupt();
        data.init_rx();
        data.init_tx();

        println!("CMD: 0x{:x}", header.hdr().command());
        println!("STS: 0x{:x}", header.hdr().status());

        true
    }*/
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

    fn send_test(&mut self) {
        let a = &[0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x21, 0xcc, 0xc0, 0x6b, 0x9b, 0x08, 0x06, 0x00, 0x01,
                            0x08  , 0x00, 0x06, 0x04, 0x00, 0x01, 0x00, 0x21, 0xcc, 0xc0, 0x6b, 0x9b, 0xc0, 0xa8, 0x01, 0x71,
                            0xff  , 0xff, 0xff, 0xff, 0xff, 0xff, 0xc0, 0xa8, 0x01, 0x71];

        unsafe {
            a.as_ptr().copy_to(BUF, a.len());
        }

        self.tx_ring[self.tx_cur as usize].addr = unsafe {
            VirtAddr(BUF as usize).to_phys_pagewalk().unwrap().0 as u64
        };
        self.tx_ring[self.tx_cur as usize].length = 42;
        self.tx_ring[self.tx_cur as usize].cmd = 0b1011;
        self.tx_ring[self.tx_cur as usize].status.bits = 0;

        let old_cur = self.tx_cur;
        self.tx_cur = (self.tx_cur + 1) % E1000_NUM_TX_DESCS as u32;

        self.addr.write(Regs::TxDescTail, self.tx_cur);

        let status = &self.tx_ring[self.tx_cur as usize].status as *const TStatus;

        unsafe {
            while status.read_volatile().bits  == 0 {}
        }

        unsafe {
            println!("Send Status: 0x{:x}", status.read_volatile().bits);
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

    fn dev_hdr(&self) -> &PciHeader0 {
        if let PciHeader::Type0(hdr) = self.hdr.as_ref().unwrap() {
            hdr
        } else {
            panic!("Invalid header")
        }
    }

    fn hdr(&self) -> &PciHeader {
        self.hdr.as_ref().expect("Invalid hdr")
    }

    fn link_up(&self) {
        self.addr.flag(Regs::Ctrl, ECtl::SLU.bits, true);
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
    }

    fn enable_interrupt(&mut self) {
        let data = self.hdr().hdr();
        let pin = data.interrupt_pin();

        let int = crate::drivers::acpi::pci_map::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        if let Some(p) = int {
            println!("[ E1000 ] Using interrupt: {}", p);
            //self.addr.write(Regs::IMask, 0xff & !4);
            //self.addr.read(Regs::ICause);

            set_irq_dest(p as u8, p as u8 + 32);
            set_handler(p as usize + 32, e1000_handler);
            //mask_int(p as u8, false);

            //self.addr.write(Regs::IMask, 0xff);
            let mask: u32 = 0b00000000010001111101001011110111;
            self.addr.write(Regs::IMaskClr, mask);
//          //  self.addr.write(Regs::IMask, 0xffff);
            self.addr.write(Regs::IMask, mask);
            self.addr.read(Regs::ICause);
            //self.hdr.as_ref().unwrap().hdr().write_interrupt_line(20);

            println!("ICR: 0x{:x}", self.addr.read_raw(0x00C4));
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

        self.addr.write(Regs::RCtrl, flags.bits);
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

        let mut flags = TCtl::EN | TCtl::PSP | TCtl::RTLC;
        flags.set_collision_threshold(15);
        flags.set_collision_distance(64);

        flags.bits |= 1u32 << 28; // reserved bit

        println!("0b{:b}", flags.bits);

        self.tx_cur = 0;

        self.addr.write(Regs::TCtrl, flags.bits);
        //self.addr.write(Regs::Tipg, 0x0060200A);
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
