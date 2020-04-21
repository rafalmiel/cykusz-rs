#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arch::raw::cpuio::Port;
use crate::drivers::pci::PciDeviceHandle;
use crate::drivers::pci::{PciHeader, PciHeader0};
use crate::kernel::mm::PhysAddr;

#[repr(u16)]
enum Regs {
    Ctrl = 0x0000,
    Status = 0x0008,
    Eeprom = 0x0014,
    CtrlExt = 0x0018,
    IMask = 0x00D0,
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

    Rdtr = 0x2820, // RX Delay Timer Register
    RxDCtrl = 0x3828, // RX Descriptor Control
    Radv = 0x282C, // RX Int. Absolute Delay Timer
    Rsrpd = 0x2C00, // RX Small Packet Detect Interrupt

    Tipg = 0x0410, // Transmit Inter Packet Gap
}

bitflags! {
    pub struct ECtl: u8 {
        const SLU           = 0x40;         // Set Link Up
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
    }
}

bitflags! {
    pub struct BufferSize: u32 {
        const SIZE_256      = (3 << 16);
        const SIZE_512      = (2 << 16);
        const SIZE_1024     = (1 << 16);
        const SIZE_2048     = (0 << 16);
        const SIZE_4096     = ((3 << 16) | (1 << 25));
        const SIZE_8192     = ((2 << 16) | (1 << 25));
        const SIZE_16384    = ((1 << 16) | (1 << 25));
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
        const DD                  = (1 << 0);     // Descriptor Done
        const EC                  = (1 << 1);     // Excess Collisions
        const LC                  = (1 << 2);     // Late Collision
        const TU                  = (1 << 3);     // Transmit Underrun
    }
}

impl Default for TStatus {
    fn default() -> Self {
        TStatus {
            bits: 0
        }
    }
}

impl TCtl {
    fn set_collision_threshold(&mut self, val: u8) {
        self.bits |= (val as u32) << 4;
    }

    fn set_collision_distance(&mut self, val: u8) {
        self.bits |= (val as u32) << 12;
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

const E1000_NUM_RX_DESCS: usize = 32;
const E1000_NUM_TX_DESCS: usize = 8;

#[allow(dead_code)]
#[repr(packed)]
#[derive(Default, Copy, Clone)]
struct RxDesc {
    addr:  u64,
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

#[allow(dead_code)]
struct E1000 {
    hdr: Option<PciHeader0>,
    addr: Addr,
    has_eeprom: bool,
    mac: [u8; 6],
    rx_ring: Vec<RxDesc>,
    tx_ring: Vec<TxDesc>,
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

        self.init_rx();
        self.init_tx();

        true
    }
}

impl E1000 {
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

    fn init_rx(&mut self) {
        self.rx_ring.resize_with(E1000_NUM_RX_DESCS, || {
            RxDesc::default()
        });
    }

    fn init_tx(&mut self) {
        self.tx_ring.resize_with(E1000_NUM_TX_DESCS, || {
            TxDesc {
                addr: 0,
                length: 0,
                cso: 0,
                cmd: 0,
                status: TStatus::DD,
                css: 0,
                special: 0
            }
        });
    }
}

fn init() {
    crate::drivers::pci::register_pci_device(Box::new(E1000 {
        hdr: None,
        addr: Addr::new(),
        has_eeprom: false,
        mac: [0; 6],
        rx_ring: Vec::new(),
        tx_ring: Vec::new(),
    }));
}

module_init!(init);
