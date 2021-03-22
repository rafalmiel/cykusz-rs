use bit_field::BitField;
use mmio::VCell;

use crate::drivers::block::ahci::reg::HbaCmdHeader;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::VirtAddr;

bitflags! {
    pub struct HbaPortISReg: u32 {
        const DHRS = 1 << 0; // Device to Host Register FIS Interrupt
        const PSS = 1 << 1; // PIO Setup FIS Interrupt
        const DSS = 1 << 2; // DMA Setup FIS Interrupt
        const SDBS = 1 << 3; // Set Device Bits Interrupt
        const UFS = 1 << 4; // Unknown FIS Interrupt
        const DPS = 1 << 5; // Descriptor Processed
        const PCS = 1 << 6; // Port Connect Change Status
        const DMPS = 1 << 7; // Device Mechanical Presence Status
        const PRCS = 1 << 22; // PhyRdy Change Status
        const IPMS = 1 << 23; // Incorrect Port Multiplier Status
        const OFS = 1 << 24; // Overflow Status
        const INFS = 1 << 26; // Interface Not-fatal Error Status
        const IFS = 1 << 27; // Interface Fatal Error Status
        const HBDS = 1 << 28; // Host Bus Data Error Status
        const HBFS = 1 << 29; // Host Bus Fatal Error Status
        const TFES = 1 << 30; // Task File Error Status
        const CPDS = 1 << 31; // Cold Port Detect Status
    }
}

bitflags! {
    pub struct HbaPortIEReg: u32 {
        const DHRE = 1 << 0; // Device to Host Register FIS Interrupt
        const PSE = 1 << 1; // PIO Setup FIS Interrupt
        const DSE = 1 << 2; // DMA Setup FIS Interrupt
        const SDBE = 1 << 3; // Set Device Bits Interrupt
        const UFE = 1 << 4; // Unknown FIS Interrupt
        const DPE = 1 << 5; // Descriptor Processed
        const PCE = 1 << 6; // Port Connect Change Status
        const DMPE = 1 << 7; // Device Mechanical Presence Status
        const PRCE = 1 << 22; // PhyRdy Change Status
        const IPME = 1 << 23; // Incorrect Port Multiplier Status
        const OFE= 1 << 24; // Overflow Status
        const INFE = 1 << 26; // Interface Not-fatal Error Status
        const IFE = 1 << 27; // Interface Fatal Error Status
        const HBDE = 1 << 28; // Host Bus Data Error Status
        const HBFE = 1 << 29; // Host Bus Fatal Error Status
        const TFEE = 1 << 30; // Task File Error Status
        const CPDE = 1 << 31; // Cold Port Detect Status
    }
}

bitflags! {
    pub struct HbaPortCmdReg: u32 {
        const ST = 1 << 0; // Start
        const SUD = 1 << 1; // Spin-Up Device
        const POD = 1 << 2; // Power On Device
        const CLO = 1 << 3; // Command List Override
        const FRE = 1 << 4; // FIS Receive Enable
        const MPSS = 1 << 13; // Mechanical Presence Switch State
        const FR = 1 << 14; // FIS Receive Running
        const CR = 1 << 15; // Command List Running
        const CPS = 1 << 16; // Cold Presence State
        const PMA = 1 << 17; // Port Multiplier Attached
        const HPCP = 1 << 18; // Hot Plug Capable Port
        const MSPC = 1 << 19; // Mechanical Presence Switch Attached to Port
        const CPD = 1 << 20; // Cold Presence Detection
        const ESP = 1 << 21; // External SATA Port
        const FBSCP = 1 << 22; // FIS-based Switching Capable Port
        const APSTE = 1 << 23; // Automatic Partial to Slumber Transition Enabled
        const ATAPI = 1 << 24; // Device is ATAPI
        const DLAE = 1 << 25; // Drive LED on ATAPI Enable
        const ALPE = 1 << 26; // Aggressive Link Power Management Enable
        const ASP = 1 << 27; // Aggressive Slumber / Partial
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum HbaPortCmdRegIcc {
    Idle = 0,
    Active = 1,
    Partial = 2,
    Slumber = 6,
    DevSleep = 8,
}

impl HbaPortCmdReg {
    pub fn current_command_slot(&self) -> usize {
        self.bits().get_bits(8..=12) as usize
    }

    pub fn interface_communication_control(&self) -> HbaPortCmdRegIcc {
        match self.bits().get_bits(28..=31) {
            0 => HbaPortCmdRegIcc::Idle,
            1 => HbaPortCmdRegIcc::Active,
            2 => HbaPortCmdRegIcc::Partial,
            6 => HbaPortCmdRegIcc::Slumber,
            8 => HbaPortCmdRegIcc::DevSleep,
            v => panic!("Invalid HbaPortCmdRegIcc {}", v),
        }
    }

    pub fn set_interface_communication_control(&mut self, v: HbaPortCmdRegIcc) {
        self.bits.set_bits(28..=31, v as u32);
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortTfdReg(pub u32);

pub enum HbaPortTfdRegStatus {
    Err,
    Drq,
    Bsy,
    CmdSpec(u32),
}

impl HbaPortTfdReg {
    fn error(&self) -> usize {
        self.0.get_bits(8..=15) as usize
    }

    fn status(&self) -> HbaPortTfdRegStatus {
        match self.0.get_bits(0..=7) {
            0 => HbaPortTfdRegStatus::Err,
            3 => HbaPortTfdRegStatus::Drq,
            7 => HbaPortTfdRegStatus::Bsy,
            o => HbaPortTfdRegStatus::CmdSpec(o),
        }
    }

    pub fn status_val(&self) -> u32 {
        self.0.get_bits(0..=7)
    }
}

#[derive(Debug)]
pub enum HbaPortSigRegDev {
    AhciDevNull = 0,
    AhciDevSata = 1,
    AhciDevSemb = 2,
    AhciDevPm = 3,
    AhciDevSatapi,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSigReg(u32);

impl HbaPortSigReg {
    pub fn sector_count_reg(&self) -> usize {
        self.0.get_bits(0..=7) as usize
    }

    pub fn lba_low_reg(&self) -> usize {
        self.0.get_bits(8..=15) as usize
    }

    pub fn lba_mid_reg(&self) -> usize {
        self.0.get_bits(16..=23) as usize
    }

    pub fn lba_high_reg(&self) -> usize {
        self.0.get_bits(24..=31) as usize
    }

    pub fn dev(&self) -> HbaPortSigRegDev {
        match self.0 {
            0xEB140101 => HbaPortSigRegDev::AhciDevSatapi,
            0xC33C0101 => HbaPortSigRegDev::AhciDevSemb,
            0x96690101 => HbaPortSigRegDev::AhciDevPm,
            _ => HbaPortSigRegDev::AhciDevSata,
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSstsReg(u32);

pub enum HbaPortSstsRegDet {
    None = 0,
    PresentNotE = 1,
    PresentAndE = 3,
    Offline = 4,
}

pub enum HbaPortSstsRegSpd {
    None = 0,
    Gen1 = 1,
    Gen2 = 2,
    Gen3 = 3,
}

pub enum HbaPortSstsRegIpm {
    None = 0,
    Active = 1,
    Partial = 2,
    Slumber = 6,
    DevSleep = 8,
}

impl HbaPortSstsReg {
    pub fn device_detection(&self) -> HbaPortSstsRegDet {
        match self.0.get_bits(0..=3) {
            0 => HbaPortSstsRegDet::None,
            1 => HbaPortSstsRegDet::PresentNotE,
            3 => HbaPortSstsRegDet::PresentAndE,
            4 => HbaPortSstsRegDet::Offline,
            v => panic!("Invalid HbaPortSstsRegDet {}", v),
        }
    }

    pub fn current_speed(&self) -> HbaPortSstsRegSpd {
        match self.0.get_bits(4..=7) {
            0 => HbaPortSstsRegSpd::None,
            1 => HbaPortSstsRegSpd::Gen1,
            2 => HbaPortSstsRegSpd::Gen2,
            3 => HbaPortSstsRegSpd::Gen3,
            v => panic!("Invalid HbaPortSstsRegSpd {}", v),
        }
    }

    pub fn interface_power_management(&self) -> HbaPortSstsRegIpm {
        match self.0.get_bits(8..=11) {
            0 => HbaPortSstsRegIpm::None,
            1 => HbaPortSstsRegIpm::Active,
            2 => HbaPortSstsRegIpm::Partial,
            6 => HbaPortSstsRegIpm::Slumber,
            8 => HbaPortSstsRegIpm::DevSleep,
            v => panic!("Invalid HbaPortSstsRegIpm {}", v),
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSctlReg(u32);

bitflags! {
    pub struct HbaPortSctlRegIpm: u32 {
        const NO_PARTIAL = 1;
        const NO_SLUMBER = 2;
        const NO_DEVSLEEP = 4;
    }
}

#[repr(u32)]
pub enum HbaPortSctlRegSpd {
    NoRestriction = 0,
    LimitToGen1 = 1,
    LimitToGen2 = 2,
    LimitToGen3 = 3,
}

#[repr(u32)]
pub enum HbaPortSctlRegDet {
    NoDetection = 0,
    InitInterface = 1,
    Disable = 4,
}

impl HbaPortSctlReg {
    pub fn select_power_management(&self) -> usize {
        self.0.get_bits(12..=15) as usize
    }

    pub fn port_multiplier_port(&self) -> usize {
        self.0.get_bits(16..=19) as usize
    }

    pub fn interface_power_mgmt_trans_allowed(&self) -> HbaPortSctlRegIpm {
        HbaPortSctlRegIpm::from_bits(self.0.get_bits(8..=11)).expect("Invalid HbaPortSctlRegIpm")
    }

    pub fn set_interface_power_mgmt_trans_allowed(&mut self, r: HbaPortSctlRegIpm) {
        self.0.set_bits(8..=11, r.bits());
    }

    pub fn speed_allowed(&self) -> HbaPortSctlRegSpd {
        match self.0.get_bits(4..=7) {
            0 => HbaPortSctlRegSpd::NoRestriction,
            1 => HbaPortSctlRegSpd::LimitToGen1,
            2 => HbaPortSctlRegSpd::LimitToGen2,
            3 => HbaPortSctlRegSpd::LimitToGen3,
            v => panic!("Invalid HbaPortSctlRegSpd {}", v),
        }
    }

    pub fn set_speed_allowed(&mut self, v: HbaPortSctlRegSpd) {
        self.0.set_bits(4..=7, v as u32);
    }

    pub fn device_detection(&self) -> HbaPortSctlRegDet {
        match self.0.get_bits(0..=3) {
            0 => HbaPortSctlRegDet::NoDetection,
            1 => HbaPortSctlRegDet::InitInterface,
            4 => HbaPortSctlRegDet::Disable,
            v => panic!("Invalid HbaPortSctlRegDet {}", v),
        }
    }

    pub fn set_device_detection(&mut self, v: HbaPortSctlRegDet) {
        self.0.set_bits(0..=3, v as u32);
    }
}

#[repr(u32)]
pub enum HbaPortSerrErr {
    I = 0,  // Recovered Data Integrity Error
    M = 1,  // Recovered Communications Error
    T = 8,  // Transient Data Integrity Error
    C = 9,  // Persistent Communication or Data Integrity Error
    P = 10, // Protocol Error
    E = 11, // Internal Error
}

#[repr(u32)]
pub enum HbaPortSerrDiag {
    N = 0,  // PhyRdy Change
    I = 1,  // Phy Internal Error
    W = 2,  // Comm Wake
    B = 3,  // 10B to 8B Decode Error
    D = 4,  // Disparity Error
    C = 5,  // CRC Error
    H = 6,  // Handshake Error
    S = 7,  // Link Sequence Error
    T = 8,  // Transport state transition error
    F = 9,  // Unknown FIS Type
    X = 10, // Exchanged
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSerrReg(u32);

impl HbaPortSerrReg {
    pub fn error(&self) -> HbaPortSerrErr {
        match self.0.get_bits(0..=15) {
            0 => HbaPortSerrErr::I,
            1 => HbaPortSerrErr::M,
            8 => HbaPortSerrErr::T,
            9 => HbaPortSerrErr::C,
            10 => HbaPortSerrErr::P,
            11 => HbaPortSerrErr::E,
            v => panic!("Invalid HbaPortSerrErr {}", v),
        }
    }

    pub fn set_error(&mut self, reg: HbaPortSerrErr) {
        self.0.set_bits(0..=15, reg as u32);
    }

    pub fn diagnostics(&self) -> HbaPortSerrDiag {
        match self.0.get_bits(16..=31) {
            0 => HbaPortSerrDiag::N,
            1 => HbaPortSerrDiag::I,
            2 => HbaPortSerrDiag::W,
            3 => HbaPortSerrDiag::B,
            4 => HbaPortSerrDiag::D,
            5 => HbaPortSerrDiag::C,
            6 => HbaPortSerrDiag::H,
            7 => HbaPortSerrDiag::S,
            8 => HbaPortSerrDiag::T,
            9 => HbaPortSerrDiag::F,
            10 => HbaPortSerrDiag::X,
            v => panic!("Invalid HbaPortSerrDiag {}", v),
        }
    }

    pub fn set_diagnostics(&mut self, reg: HbaPortSerrDiag) {
        self.0.set_bits(16..=31, reg as u32);
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortFbs(u32);

impl HbaPortFbs {
    pub fn enable(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_enable(&mut self, v: bool) {
        self.0.set_bit(0, v);
    }

    pub fn device_error_clear(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_device_error_clear(&mut self, v: bool) {
        self.0.set_bit(1, v);
    }

    pub fn single_device_error(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn device_to_issue(&self) -> usize {
        self.0.get_bits(8..=11) as usize
    }

    pub fn set_device_to_issue(&mut self, v: usize) {
        self.0.set_bits(8..=11, v as u32);
    }

    pub fn active_device_optimization(&self) -> usize {
        self.0.get_bits(12..=15) as usize
    }

    pub fn device_with_error(&self) -> usize {
        self.0.get_bits(16..=19) as usize
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortDevslp(u32);

impl HbaPortDevslp {
    pub fn aggressive_device_sleep_enable(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_aggressive_device_sleep_enable(&mut self, v: bool) {
        self.0.set_bit(0, v);
    }

    pub fn device_sleep_present(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_device_sleep_present(&mut self, v: bool) {
        self.0.set_bit(1, v);
    }

    pub fn device_sleep_exit_timeout(&self) -> usize {
        self.0.get_bits(2..=9) as usize
    }

    pub fn set_device_sleep_exit_timeout(&mut self, v: usize) {
        self.0.set_bits(2..=9, v as u32);
    }

    pub fn minimum_device_sleep_assertion_time(&self) -> usize {
        self.0.get_bits(10..=14) as usize
    }

    pub fn set_minimum_device_sleep_assertion_time(&mut self, v: usize) {
        self.0.set_bits(10..=14, v as u32);
    }

    pub fn device_sleep_idle_timeout(&self) -> usize {
        self.0.get_bits(15..=24) as usize
    }

    pub fn set_device_sleep_idle_timeout(&mut self, v: usize) {
        self.0.set_bits(15..=14, v as u32);
    }

    pub fn dito_multiplier(&self) -> usize {
        self.0.get_bits(25..=28) as usize
    }
}

#[repr(C, packed)]
pub struct HbaPort {
    clb: VCell<PhysAddr>,
    fb: VCell<PhysAddr>,
    is: VCell<HbaPortISReg>,
    ie: VCell<HbaPortIEReg>,
    cmd: VCell<HbaPortCmdReg>,
    _rsv: u32,
    tfd: VCell<HbaPortTfdReg>,
    sig: VCell<HbaPortSigReg>,
    ssts: VCell<HbaPortSstsReg>,
    sctl: VCell<HbaPortSctlReg>,
    serr: VCell<HbaPortSerrReg>,
    sact: VCell<u32>,
    ci: VCell<u32>,
    sntf: VCell<u32>,
    fbs: VCell<HbaPortFbs>,
    devslp: VCell<HbaPortDevslp>,
    _rsv1: [u32; 10],
    vendor: [u32; 4],
}

impl HbaPort {
    pub fn start_cmd(&mut self) {
        while self.cmd().contains(HbaPortCmdReg::CR) {}

        self.set_cmd(self.cmd() | (HbaPortCmdReg::FRE | HbaPortCmdReg::ST));
    }

    pub fn stop_cmd(&mut self) {
        let mut cmd = self.cmd();

        cmd.remove(HbaPortCmdReg::FRE | HbaPortCmdReg::ST);

        self.set_cmd(cmd);

        while self.cmd().intersects(HbaPortCmdReg::FR | HbaPortCmdReg::CR) {}
    }

    pub fn clb(&self) -> PhysAddr {
        unsafe { self.clb.get() }
    }

    pub fn set_clb(&mut self, addr: PhysAddr) {
        unsafe {
            self.clb.set(addr);
        }
    }

    pub fn cmd_header_at(&self, idx: usize) -> &mut HbaCmdHeader {
        unsafe {
            (self.clb().to_mapped() + core::mem::size_of::<HbaCmdHeader>() * idx)
                .read_mut::<HbaCmdHeader>()
        }
    }

    pub fn fb(&self) -> PhysAddr {
        unsafe { self.fb.get() }
    }

    pub fn set_fb(&mut self, addr: PhysAddr) {
        unsafe {
            self.fb.set(addr);
        }
    }

    pub fn is(&self) -> HbaPortISReg {
        unsafe { self.is.get() }
    }

    pub fn set_is(&mut self, reg: HbaPortISReg) {
        unsafe { self.is.set(reg) }
    }

    pub fn ie(&self) -> HbaPortIEReg {
        unsafe { self.ie.get() }
    }

    pub fn set_ie(&mut self, reg: HbaPortIEReg) {
        unsafe { self.ie.set(reg) }
    }

    pub fn cmd(&self) -> HbaPortCmdReg {
        unsafe { self.cmd.get() }
    }

    pub fn set_cmd(&mut self, reg: HbaPortCmdReg) {
        unsafe { self.cmd.set(reg) }
    }

    pub fn tfd(&self) -> HbaPortTfdReg {
        unsafe { self.tfd.get() }
    }

    pub fn sig(&self) -> HbaPortSigReg {
        unsafe { self.sig.get() }
    }

    pub fn ssts(&self) -> HbaPortSstsReg {
        unsafe { self.ssts.get() }
    }

    pub fn sctl(&self) -> HbaPortSctlReg {
        unsafe { self.sctl.get() }
    }

    pub fn set_sctl(&mut self, reg: HbaPortSctlReg) {
        unsafe { self.sctl.set(reg) }
    }

    pub fn serr(&self) -> HbaPortSerrReg {
        unsafe { self.serr.get() }
    }

    pub fn set_serr(&mut self, reg: HbaPortSerrReg) {
        unsafe { self.serr.set(reg) }
    }

    pub fn sact(&self) -> u32 {
        unsafe { self.sact.get() }
    }

    pub fn set_sact(&mut self, v: u32) {
        unsafe { self.sact.set(v) }
    }

    pub fn ci(&self) -> u32 {
        unsafe { self.ci.get() }
    }

    pub fn set_ci(&mut self, v: u32) {
        unsafe { self.ci.set(v) }
    }

    pub fn sntf(&self) -> u32 {
        unsafe { self.sntf.get() }
    }

    pub fn set_sntf(&mut self, v: u32) {
        unsafe { self.sntf.set(v) }
    }

    pub fn fbs(&self) -> HbaPortFbs {
        unsafe { self.fbs.get() }
    }

    pub fn set_fbs(&mut self, reg: HbaPortFbs) {
        unsafe { self.fbs.set(reg) }
    }

    pub fn devslp(&self) -> HbaPortDevslp {
        unsafe { self.devslp.get() }
    }

    pub fn set_devslp(&mut self, reg: HbaPortDevslp) {
        unsafe { self.devslp.set(reg) }
    }

    pub fn vendor(&self) -> &[u32] {
        unsafe { &self.vendor }
    }
}
