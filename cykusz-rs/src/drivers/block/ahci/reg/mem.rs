use bit_field::BitField;
use mmio::VCell;

use super::HbaPort;

bitflags! {
    pub struct HbaMemCapReg: u32 {
        const SXS           = 1 << 5;  // Supports External SATA
        const EMS           = 1 << 6;  // Enclosure Management Supported
        const CCCS          = 1 << 7;  // Command Completion Coalescing Supported
        const PSC           = 1 << 13; // Partial State Capable
        const SSC           = 1 << 14; // Slumber State Capable
        const PMD           = 1 << 15; // PIO Multiple DRQ Block
        const FBSS          = 1 << 16; // FIS-based Switching Supported
        const SPM           = 1 << 17; // Supports Port Multiplier
        const SAM           = 1 << 18; // Supports AHCI mode only
        const SCLO          = 1 << 24; // Supports Command List Override
        const SAL           = 1 << 25; // Supports Activity LED
        const SALP          = 1 << 26; // Supports Aggressive Link Power Mgmt
        const SSS           = 1 << 27; // Supports Staggered Spin-up
        const SMPS          = 1 << 28; // Supports Mechanical Presence Switch
        const SSNTF         = 1 << 29; // Supports SNotification Register
        const SNCQ          = 1 << 30; // Supports Native Command Queuing
        const S64A          = 1 << 31; // Supports 64-bit Addressing
    }
}

pub enum HbaMemCapRegISpeed {
    Gen1,
    Gen2,
    Gen3,
}

impl HbaMemCapReg {
    pub fn num_ports(&self) -> usize {
        self.bits.get_bits(0..=4) as usize
    }

    pub fn num_cmd_ports(&self) -> usize {
        self.bits.get_bits(8..=12) as usize
    }

    pub fn ispeed(&self) -> HbaMemCapRegISpeed {
        match self.bits.get_bits(20..=23) {
            0b0001 => HbaMemCapRegISpeed::Gen1,
            0b0010 => HbaMemCapRegISpeed::Gen2,
            0b0011 => HbaMemCapRegISpeed::Gen3,
            _ => panic!("Invalid Interface Speed Support Field"),
        }
    }
}

bitflags! {
    pub struct HbaMemGhcReg: u32 {
        const HR =   1 << 0;  // HBA Reset
        const IE =   1 << 1;  // Interrupt Enable
        const MRSM = 1 << 2;  // MSI Revert to Single Message
        const AE =   1 << 31; // AHCI Enable
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaMemCCCCtlReg(u32);

impl HbaMemCCCCtlReg {
    pub fn enable(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_enable(&mut self, e: bool) -> HbaMemCCCCtlReg {
        HbaMemCCCCtlReg(*self.0.set_bit(0, e))
    }

    pub fn interrupt(&self) -> usize {
        self.0.get_bits(3..=7) as usize
    }

    pub fn command_completions(&self) -> usize {
        self.0.get_bits(8..=15) as usize
    }

    pub fn set_command_completions(&mut self, cmds: usize) -> HbaMemCCCCtlReg {
        HbaMemCCCCtlReg(*self.0.set_bits(8..=15, cmds as u32))
    }

    pub fn timeout_value(&self) -> usize {
        self.0.get_bits(16..=31) as usize
    }

    pub fn set_timeout_value(&mut self, timeout: usize) -> HbaMemCCCCtlReg {
        HbaMemCCCCtlReg(*self.0.set_bits(16..=31, timeout as u32))
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaMemEmLocReg(u32);

impl HbaMemEmLocReg {
    pub fn buffer_size(&self) -> usize {
        self.0.get_bits(0..=15) as usize
    }

    pub fn offset(&self) -> usize {
        self.0.get_bits(16..=31) as usize
    }
}

bitflags! {
    pub struct HbaMemEmlCtlReg: u32{
        const STS_MR =      1 << 0;  // Message Received
        const CTL_TM =      1 << 8;  // Transmit Message
        const CTL_RST =     1 << 9;  // Reset
        const SUPP_LED =    1 << 16; // LED Message Types
        const SUPP_SAFTE =  1 << 17; // SAF-TE Enclosure Management Messages
        const SUPP_SES2 =   1 << 18; // SES-2 Enclosure Management Messages
        const SUPP_SGPIO =  1 << 19; // SGPIO Enclosure Management Messages
        const ATTR_SMB =    1 << 24; // Single Message Buffer
        const ATTR_XMT =    1 << 25; // Transmit Only
        const ATTR_ALHD =   1 << 26; // Activity LED Hardware Driven
        const ATTR_PM =     1 << 27; // Port Multiplier Support
    }
}

bitflags! {
    pub struct HbaMemCap2Reg: u32 {
        const BOH   = 1 << 0; // BIOS/OS Handoff
        const NVMP  = 1 << 1; // NVMHCI Present
        const APST  = 1 << 2; // Automatic Partial to Slumber Transitions
        const SDS   = 1 << 3; // Supports Device Sleep
        const SADM  = 1 << 4; // Supports Aggressive Device Sleep management
        const DESO  = 1 << 5; // DevSleep Entrance from Slumber Only
    }
}

bitflags! {
    pub struct HbaMemBohcReg: u32 {
        const BOS =     1 << 0; // BIOS Owned Semaphore
        const OOS =     1 << 1; // OS Owned Semaphore
        const SOOE =    1 << 2; // SMI on OS Ownership Change Enable
        const OOC =     1 << 3; // OS Ownership Change
        const BB =      1 << 4; // BIOS Busy
    }
}

#[repr(C, packed)]
pub struct HbaMem {
    cap: VCell<HbaMemCapReg>,
    ghc: VCell<HbaMemGhcReg>,
    is: VCell<u32>,
    pi: VCell<u32>,
    vs: VCell<u32>,
    ccc_ctl: VCell<HbaMemCCCCtlReg>,
    ccc_pts: VCell<u32>,
    em_loc: VCell<HbaMemEmLocReg>,
    em_ctl: VCell<HbaMemEmlCtlReg>,
    cap2: VCell<HbaMemCap2Reg>,
    bohc: VCell<HbaMemBohcReg>,
    _rsv: [u8; 0xa0 - 0x2c],
    vendor: [u8; 0x100 - 0xa0],
}

impl HbaMem {
    pub fn cap(&self) -> HbaMemCapReg {
        unsafe { self.cap.get() }
    }

    pub fn ghc(&self) -> HbaMemGhcReg {
        unsafe { self.ghc.get() }
    }

    pub fn set_ghc(&mut self, ghc: HbaMemGhcReg) {
        unsafe { self.ghc.set(ghc) }
    }

    pub fn is(&self) -> u32 {
        unsafe { self.is.get() }
    }

    pub fn set_is(&mut self, is: u32) {
        unsafe { self.is.set(is) }
    }

    pub fn pi(&self) -> u32 {
        unsafe { self.pi.get() }
    }

    pub fn vs(&self) -> u32 {
        unsafe { self.vs.get() }
    }

    pub fn ccc_ctl(&self) -> HbaMemCCCCtlReg {
        unsafe { self.ccc_ctl.get() }
    }

    pub fn set_ccc_ctl(&mut self, reg: HbaMemCCCCtlReg) {
        unsafe { self.ccc_ctl.set(reg) }
    }

    pub fn ccc_pts(&self) -> u32 {
        unsafe { self.ccc_pts.get() }
    }

    pub fn set_ccc_pts(&mut self, ports: u32) {
        unsafe { self.ccc_pts.set(ports) }
    }

    pub fn em_loc(&self) -> HbaMemEmLocReg {
        unsafe { self.em_loc.get() }
    }

    pub fn em_ctl(&self) -> HbaMemEmlCtlReg {
        unsafe { self.em_ctl.get() }
    }

    pub fn set_em_ctl(&mut self, reg: HbaMemEmlCtlReg) {
        unsafe { self.em_ctl.set(reg) }
    }

    pub fn cap2(&self) -> HbaMemCap2Reg {
        unsafe { self.cap2.get() }
    }

    pub fn bohc(&self) -> HbaMemBohcReg {
        unsafe { self.bohc.get() }
    }

    pub fn set_bohc(&mut self, reg: HbaMemBohcReg) {
        unsafe { self.bohc.set(reg) }
    }

    pub fn vendor(&self) -> &[u8] {
        &self.vendor
    }

    pub fn port(&self, at: usize) -> &HbaPort {
        return unsafe {
            &*((self as *const HbaMem).offset(1) as *const HbaPort).offset(at as isize)
        };
    }

    pub fn port_mut(&mut self, at: usize) -> &mut HbaPort {
        return unsafe {
            &mut *((self as *mut HbaMem).offset(1) as *mut HbaPort).offset(at as isize)
        };
    }
}
