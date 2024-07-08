use crate::drivers::block::ahci::reg::HbaCmdHeader;
use crate::kernel::block::BlockDev;
use crate::kernel::mm::PhysAddr;
use bit_field::BitField;
use tock_registers::fields::FieldValue;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::{InMemoryRegister, ReadWrite};
use tock_registers::{register_bitfields, LocalRegisterCopy, register_structs};

register_bitfields! [
    u32,

    pub HbaPortISReg [
        DHRS 0, // Device to Host Register FIS Interrupt
        PSS  1, // PIO Setup FIS Interrupt
        DSS  2, // DMA Setup FIS Interrupt
        SDBS 3, // Set Device Bits Interrupt
        UFS  4, // Unknown FIS Interrupt
        DPS  5, // Descriptor Processed
        PCS  6, // Port Connect Change Status
        DMPS 7, // Device Mechanical Presence Status
        PRCS 22, // PhyRdy Change Status
        IPMS 23, // Incorrect Port Multiplier Status
        OFS  24, // Overflow Status
        INFS 26, // Interface Not-fatal Error Status
        IFS  27, // Interface Fatal Error Status
        HBDS 28, // Host Bus Data Error Status
        HBFS 29, // Host Bus Fatal Error Status
        TFES 30, // Task File Error Status
        CPDS 31, // Cold Port Detect Status
    ]
];

register_bitfields! [
    u32,

    pub HbaPortIEReg [
        DHRE 0, // Device to Host Register FIS Interrupt
        PSE  1, // PIO Setup FIS Interrupt
        DSE  2, // DMA Setup FIS Interrupt
        SDBE 3, // Set Device Bits Interrupt
        UFE  4, // Unknown FIS Interrupt
        DPE  5, // Descriptor Processed
        PCE  6, // Port Connect Change Status
        DMPE 7, // Device Mechanical Presence Status
        PRCE 22, // PhyRdy Change Status
        IPME 23, // Incorrect Port Multiplier Status
        OFE  24, // Overflow Status
        INFE 26, // Interface Not-fatal Error Status
        IFE  27, // Interface Fatal Error Status
        HBDE 28, // Host Bus Data Error Status
        HBFE 29, // Host Bus Fatal Error Status
        TFEE 30, // Task File Error Status
        CPDE 31, // Cold Port Detect Status
    ]
];

register_bitfields! [
    u32,

    pub HbaPortCmdRegDef [
        ST    OFFSET(0) NUMBITS(1) [], // Start
        SUD   OFFSET(1) NUMBITS(1) [], // Spin-Up Device
        POD   OFFSET(2) NUMBITS(1) [], // Power On Device
        CLO   OFFSET(3) NUMBITS(1) [], // Command List Override
        FRE   OFFSET(4) NUMBITS(1) [], // FIS Receive Enable
        CURRENT_COMMAND_SLOT OFFSET(8) NUMBITS(5) [],
        MPSS  OFFSET(13) NUMBITS(1) [], // Mechanical Presence Switch State
        FR    OFFSET(14) NUMBITS(1) [], // FIS Receive Running
        CR    OFFSET(15) NUMBITS(1) [], // Command List Running
        CPS   OFFSET(16) NUMBITS(1) [], // Cold Presence State
        PMA   OFFSET(17) NUMBITS(1) [], // Port Multiplier Attached
        HPCP  OFFSET(18) NUMBITS(1) [], // Hot Plug Capable Port
        MSPC  OFFSET(19) NUMBITS(1) [], // Mechanical Presence Switch Attached to Port
        CPD   OFFSET(20) NUMBITS(1) [], // Cold Presence Detection
        ESP   OFFSET(21) NUMBITS(1) [], // External SATA Port
        FBSCP OFFSET(22) NUMBITS(1) [], // FIS-based Switching Capable Port
        APSTE OFFSET(23) NUMBITS(1) [], // Automatic Partial to Slumber Transition Enabled
        ATAPI OFFSET(24) NUMBITS(1) [], // Device is ATAPI
        DLAE  OFFSET(25) NUMBITS(1) [], // Drive LED on ATAPI Enable
        ALPE  OFFSET(26) NUMBITS(1) [], // Aggressive Link Power Management Enable
        ASP   OFFSET(27) NUMBITS(1) [], // Aggressive Slumber / Partial
        INTERFACE_COMMUNICATION_CONTROL OFFSET(28) NUMBITS(4) [
            Idle = 0,
            Active = 1,
            Partial = 2,
            Slumber = 6,
            DevSleep = 8
        ]
    ]
];

pub struct HbaPortCmdReg(LocalRegisterCopy<u32, HbaPortCmdRegDef::Register>);

impl HbaPortCmdReg {
    pub fn current_command_slot(&self) -> usize {
        self.0.read(HbaPortCmdRegDef::CURRENT_COMMAND_SLOT) as usize
    }

    pub fn interface_communication_control(
        &self,
    ) -> HbaPortCmdRegDef::INTERFACE_COMMUNICATION_CONTROL::Value {
        self.0
            .read_as_enum(HbaPortCmdRegDef::INTERFACE_COMMUNICATION_CONTROL)
            .expect("Invalid interface_communication_control")
    }

    pub fn set_interface_communication_control(
        &mut self,
        v: HbaPortCmdRegDef::INTERFACE_COMMUNICATION_CONTROL::Value,
    ) {
        self.0
            .modify(HbaPortCmdRegDef::INTERFACE_COMMUNICATION_CONTROL.val(v as u32))
    }
}

register_bitfields! [
    u32,

    HbaPortTfdReg [
        ERR OFFSET(0) NUMBITS(1),
        DRQ OFFSET(3) NUMBITS(1),
        BSY OFFSET(7) NUMBITS(1),
        ERROR OFFSET(8) NUMBITS(8),
    ]
];

#[derive(Debug)]
pub enum HbaPortSigRegDev {
    AhciDevNull = 0,
    AhciDevSata = 1,
    AhciDevSemb = 2,
    AhciDevPm = 3,
    AhciDevSatapi,
}

register_bitfields! [
    u32,

    HbaPortSigRegDef [
        SECTOR_COUNT OFFSET(0) NUMBITS(8) [],
        LBA_LOW OFFSET(8) NUMBITS(8) [],
        LBA_MID OFFSET(16) NUMBITS(8) [],
        LBA_HIGH OFFSET(24) NUMBITS(8) [],
        DEV OFFSET(0) NUMBITS(32) [
            AhciDevSatapi = 0xEB140101,
            AhciDevSemb = 0xC33C0101,
            AhciDevPm = 0x96690101,
        ],
    ]
];

pub struct HbaPortSigReg(LocalRegisterCopy<u32, HbaPortSigRegDef::Register>);

impl HbaPortSigReg {
    pub fn sector_count_reg(&self) -> usize {
        self.0.read(HbaPortSigRegDef::SECTOR_COUNT) as usize
    }

    pub fn lba_low_reg(&self) -> usize {
        self.0.read(HbaPortSigRegDef::LBA_LOW) as usize
    }

    pub fn lba_mid_reg(&self) -> usize {
        self.0.read(HbaPortSigRegDef::LBA_MID) as usize
    }

    pub fn lba_high_reg(&self) -> usize {
        self.0.read(HbaPortSigRegDef::LBA_HIGH) as usize
    }

    pub fn dev(&self) -> HbaPortSigRegDev {
        match self.0.read_as_enum(HbaPortSigRegDef::DEV) {
            Some(HbaPortSigRegDef::DEV::Value::AhciDevSatapi) => HbaPortSigRegDev::AhciDevSemb,
            Some(HbaPortSigRegDef::DEV::Value::AhciDevSatapi) => HbaPortSigRegDev::AhciDevSatapi,
            Some(HbaPortSigRegDef::DEV::Value::AhciDevPm) => HbaPortSigRegDev::AhciDevPm,
            _ => HbaPortSigRegDev::AhciDevSata,
        }
    }
}

register_bitfields! [
    u32,

    pub HbaPortSstsRegDef [
        DEVICE_DETECTION OFFSET(0) NUMBITS(4) [
            None = 0,
            PresentNotE = 1,
            PresentAndE = 3,
            Offline = 4,
        ],
        CURRENT_SPEED OFFSET(4) NUMBITS(4) [
            None = 0,
            Gen1 = 1,
            Gen2 = 2,
            Gen3 = 3,
        ],
        INTERFACE_POWER_MANAGEMENT OFFSET(8) NUMBITS(4) [
            None = 0,
            Active = 1,
            Partial = 2,
            Slumber = 6,
            DevSleep = 8,
        ]
    ]
];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSstsReg(LocalRegisterCopy<u32, HbaPortSstsRegDef::Register>);

impl HbaPortSstsReg {
    pub fn device_detection(&self) -> HbaPortSstsRegDef::DEVICE_DETECTION::Value {
        self.0
            .read_as_enum(HbaPortSstsRegDef::DEVICE_DETECTION)
            .expect("Invalid Ssts device_detection")
    }

    pub fn current_speed(&self) -> HbaPortSstsRegDef::CURRENT_SPEED::Value {
        self.0
            .read_as_enum(HbaPortSstsRegDef::CURRENT_SPEED)
            .expect("Invalid Ssts current_speed")
    }

    pub fn interface_power_management(
        &self,
    ) -> HbaPortSstsRegDef::INTERFACE_POWER_MANAGEMENT::Value {
        self.0
            .read_as_enum(HbaPortSstsRegDef::INTERFACE_POWER_MANAGEMENT)
            .expect("Invalid Ssts interface_power_management")
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortSctlReg(LocalRegisterCopy<u32, HbaPortSctlRegDef::Register>);

register_bitfields! [
    u32,

    HbaPortSctlRegDef [
        DEVICE_DETECTION OFFSET(0) NUMBITS(4) [
            NoDetection = 0,
            InitInterface = 1,
            Disablee = 4,
        ],
        SPEED_ALLOWED OFFSET(4) NUMBITS(4) [
            NoRestriction = 0,
            LimitToGen1 = 1,
            LimitToGen2 = 2,
            LimitToGen3 = 3,
        ],
        IPM OFFSET(8) NUMBITS(4) [
            NoPartial = 1,
            NoSlumber = 2,
            NoDevsleep = 4,
        ],
        SELECT_POWER_MANAGEMENT OFFSET(12) NUMBITS(4) [],
        PORT_MULTIPLIER_PORT OFFSET(16) NUMBITS(4) [],
    ]
];

impl HbaPortSctlReg {
    pub fn select_power_management(&self) -> usize {
        self.0.read(HbaPortSctlRegDef::SELECT_POWER_MANAGEMENT) as usize
    }

    pub fn port_multiplier_port(&self) -> usize {
        self.0.read(HbaPortSctlRegDef::PORT_MULTIPLIER_PORT) as usize
    }

    pub fn interface_power_mgmt_trans_allowed(&self) -> HbaPortSctlRegDef::IPM::Value {
        self.0
            .read_as_enum(HbaPortSctlRegDef::IPM)
            .expect("Invalid Sctl ipm")
    }

    pub fn set_interface_power_mgmt_trans_allowed(&mut self, r: HbaPortSctlRegDef::IPM::Value) {
        self.0.modify(HbaPortSctlRegDef::IPM.val(r as u32))
    }

    pub fn speed_allowed(&self) -> HbaPortSctlRegDef::SPEED_ALLOWED::Value {
        self.0
            .read_as_enum(HbaPortSctlRegDef::SPEED_ALLOWED)
            .expect("Invalid Sctl speed_allowed")
    }

    pub fn set_speed_allowed(&mut self, v: HbaPortSctlRegDef::SPEED_ALLOWED::Value) {
        self.0
            .modify(HbaPortSctlRegDef::SPEED_ALLOWED.val(v as u32))
    }

    pub fn device_detection(&self) -> HbaPortSctlRegDef::DEVICE_DETECTION::Value {
        self.0
            .read_as_enum(HbaPortSctlRegDef::DEVICE_DETECTION)
            .expect("Invalid Sctl device_detection")
    }

    pub fn set_device_detection(&mut self, v: HbaPortSctlRegDef::DEVICE_DETECTION::Value) {
        self.0
            .modify(HbaPortSctlRegDef::DEVICE_DETECTION.val(v as u32))
    }
}

register_bitfields! [
    u32,

    pub HbaPortSerr [
        ERR_I 0,  // Recovered Data Integrity Error
        ERR_M 1,  // Recovered Communications Error
        ERR_T 8,  // Transient Data Integrity Error
        ERR_C 9,  // Persistent Communication or Data Integrity Error
        ERR_P 10, // Protocol Error
        ERR_E 11, // Internal Error

        DIAG_N 16,  // PhyRdy Change
        DIAG_I 17,  // Phy Internal Error
        DIAG_W 18,  // Comm Wake
        DIAG_B 19,  // 10B to 8B Decode Error
        DIAG_D 20,  // Disparity Error
        DIAG_C 21,  // CRC Error
        DIAG_H 22,  // Handshake Error
        DIAG_S 23,  // Link Sequence Error
        DIAG_T 24,  // Transport state transition error
        DIAG_F 25,  // Unknown FIS Type
        DIAG_X 26, // Exchanged
    ]
];

register_bitfields! [
    u32,

    HbaPortFbsDef [
        ENABLE OFFSET(0) NUMBITS(1),
        DEVICE_ERROR_CLEAR OFFSET(1) NUMBITS(1),
        SINGLE_DEVICE_ERROR OFFSET(2) NUMBITS(1),
        DEVICE_TO_ISSUE OFFSET(8) NUMBITS(4),
        ACTIVE_DEVICE_OPTIMIZATION OFFSET(12) NUMBITS(4),
        DEVICE_WITH_ERROR OFFSET(16) NUMBITS(4),
    ]
];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortFbs(LocalRegisterCopy<u32, HbaPortFbsDef::Register>);

impl HbaPortFbs {
    pub fn enable(&self) -> bool {
        self.0.is_set(HbaPortFbsDef::ENABLE)
    }

    pub fn set_enable(&mut self, v: bool) {
        self.0.modify(if v {
            HbaPortFbsDef::ENABLE::SET
        } else {
            HbaPortFbsDef::ENABLE::CLEAR
        });
    }

    pub fn device_error_clear(&self) -> bool {
        self.0.is_set(HbaPortFbsDef::DEVICE_ERROR_CLEAR)
    }

    pub fn set_device_error_clear(&mut self, v: bool) {
        self.0.modify(if v {
            HbaPortFbsDef::DEVICE_ERROR_CLEAR::SET
        } else {
            HbaPortFbsDef::DEVICE_ERROR_CLEAR::CLEAR
        });
    }

    pub fn single_device_error(&self) -> bool {
        self.0.is_set(HbaPortFbsDef::SINGLE_DEVICE_ERROR)
    }

    pub fn device_to_issue(&self) -> usize {
        self.0.read(HbaPortFbsDef::DEVICE_TO_ISSUE) as usize
    }

    pub fn set_device_to_issue(&mut self, v: usize) {
        self.0.modify(HbaPortFbsDef::DEVICE_TO_ISSUE.val(v as u32))
    }

    pub fn active_device_optimization(&self) -> usize {
        self.0.read(HbaPortFbsDef::ACTIVE_DEVICE_OPTIMIZATION) as usize
    }

    pub fn device_with_error(&self) -> usize {
        self.0.read(HbaPortFbsDef::DEVICE_WITH_ERROR) as usize
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaPortDevslp(LocalRegisterCopy<u32, HbaPortDevslpDef::Register>);

register_bitfields! [
    u32,

    HbaPortDevslpDef [
        AGGRESSIVE_DEV_SLEEP_ENABLE OFFSET(0) NUMBITS(1),
        DEVICE_SLEEP_PRESENT OFFSET(1) NUMBITS(1),
        DEVICE_SLEEP_EXIT_TIMEOUT OFFSET(2) NUMBITS(8),
        DEVICE_SLEEP_ASSERTION_TIME OFFSET(10) NUMBITS(5),
        DEVICE_SLEEP_IDLE_TIMEOUT OFFSET(15) NUMBITS(10),
        DITO_MULTIPLIER OFFSET(25) NUMBITS(4),
    ]
];

impl HbaPortDevslp {
    pub fn aggressive_device_sleep_enable(&self) -> bool {
        self.0.is_set(HbaPortDevslpDef::AGGRESSIVE_DEV_SLEEP_ENABLE)
    }

    pub fn set_aggressive_device_sleep_enable(&mut self, v: bool) {
        self.0.modify(if v {
            HbaPortDevslpDef::AGGRESSIVE_DEV_SLEEP_ENABLE::SET
        } else {
            HbaPortDevslpDef::AGGRESSIVE_DEV_SLEEP_ENABLE::CLEAR
        });
    }

    pub fn device_sleep_present(&self) -> bool {
        self.0.is_set(HbaPortDevslpDef::DEVICE_SLEEP_PRESENT)
    }

    pub fn set_device_sleep_present(&mut self, v: bool) {
        self.0.modify(if v {
            HbaPortDevslpDef::DEVICE_SLEEP_PRESENT::SET
        } else {
            HbaPortDevslpDef::DEVICE_SLEEP_PRESENT::CLEAR
        });
    }

    pub fn device_sleep_exit_timeout(&self) -> usize {
        self.0.read(HbaPortDevslpDef::DEVICE_SLEEP_EXIT_TIMEOUT) as usize
    }

    pub fn set_device_sleep_exit_timeout(&mut self, v: usize) {
        self.0
            .modify(HbaPortDevslpDef::DEVICE_SLEEP_EXIT_TIMEOUT.val(v as u32))
    }

    pub fn minimum_device_sleep_assertion_time(&self) -> usize {
        self.0.read(HbaPortDevslpDef::DEVICE_SLEEP_ASSERTION_TIME) as usize
    }

    pub fn set_minimum_device_sleep_assertion_time(&mut self, v: usize) {
        self.0
            .modify(HbaPortDevslpDef::DEVICE_SLEEP_ASSERTION_TIME.val(v as u32))
    }

    pub fn device_sleep_idle_timeout(&self) -> usize {
        self.0.read(HbaPortDevslpDef::DEVICE_SLEEP_IDLE_TIMEOUT) as usize
    }

    pub fn set_device_sleep_idle_timeout(&mut self, v: usize) {
        self.0
            .modify(HbaPortDevslpDef::DEVICE_SLEEP_IDLE_TIMEOUT.val(v as u32))
    }

    pub fn dito_multiplier(&self) -> usize {
        self.0.read(HbaPortDevslpDef::DITO_MULTIPLIER) as usize
    }
}

register_structs! {
    pub HbaPort {
        (0x0000 => clb: ReadWrite<u64>),
        (0x0008 => fb: ReadWrite<u64>),
        (0x0010 => is: ReadWrite<u32, HbaPortISReg::Register>),
        (0x0014 => ie: ReadWrite<u32, HbaPortIEReg::Register>),
        (0x0018 => cmd: ReadWrite<u32, HbaPortCmdRegDef::Register>),
        (0x001C => _rsv), // 4
        (0x0020 => tfd: ReadWrite<u32, HbaPortTfdReg::Register>),
        (0x0024 => sig: ReadWrite<u32, HbaPortSigRegDef::Register>),
        (0x0028 => ssts: ReadWrite<u32, HbaPortSstsRegDef::Register>),
        (0x002C => sctl: ReadWrite<u32, HbaPortSctlRegDef::Register>),
        (0x0030 => serr: ReadWrite<u32, HbaPortSerr::Register>),
        (0x0034 => sact: ReadWrite<u32>),
        (0x0038 => ci: ReadWrite<u32>),
        (0x003C => sntf: ReadWrite<u32>),
        (0x0040 => fbs: ReadWrite<u32, HbaPortFbsDef::Register>),
        (0x0044 => devslp: ReadWrite<u32, HbaPortDevslpDef::Register>),
        (0x0048 => _rsv1), // 40
        (0x0070 => vendor: [u32; 4]),
        (0x0080 => @END),
    }
}

impl HbaPort {
    pub fn start_cmd(&mut self) {
        while self.cmd().0.is_set(HbaPortCmdRegDef::CR) {}

        let mut cmd = self.cmd();
        cmd.0
            .modify(HbaPortCmdRegDef::FRE::SET + HbaPortCmdRegDef::ST::SET);

        self.set_cmd(cmd)
    }

    pub fn stop_cmd(&mut self) {
        let mut cmd = self.cmd();

        cmd.0
            .modify(HbaPortCmdRegDef::FRE::CLEAR + HbaPortCmdRegDef::ST::CLEAR);
        self.set_cmd(cmd);

        while self
            .cmd()
            .0
            .matches_any(&[HbaPortCmdRegDef::FR::SET, HbaPortCmdRegDef::CR::SET])
        {}
    }

    pub fn clb(&self) -> PhysAddr {
        PhysAddr(self.clb.get() as usize)
    }

    pub fn set_clb(&mut self, addr: PhysAddr) {
        self.clb.set(addr.0 as u64);
    }

    pub fn cmd_header_at(&self, idx: usize) -> &mut HbaCmdHeader {
        unsafe {
            (self.clb().to_virt() + core::mem::size_of::<HbaCmdHeader>() * idx)
                .read_mut::<HbaCmdHeader>()
        }
    }

    pub fn fb(&self) -> PhysAddr {
        PhysAddr(self.fb.get() as usize)
    }

    pub fn set_fb(&mut self, addr: PhysAddr) {
        self.fb.set(addr.0 as u64);
    }

    pub fn is(&self) -> LocalRegisterCopy<u32, HbaPortISReg::Register> {
        self.is.extract()
    }

    pub fn set_is(&mut self, reg: LocalRegisterCopy<u32, HbaPortISReg::Register>) {
        self.is.set(reg.get())
    }

    pub fn ie(&self) -> LocalRegisterCopy<u32, HbaPortIEReg::Register> {
        self.ie.extract()
    }

    pub fn set_ie(&mut self, reg: FieldValue<u32, HbaPortIEReg::Register>) {
        self.ie.write(reg)
    }

    pub fn cmd(&self) -> HbaPortCmdReg {
        HbaPortCmdReg(self.cmd.extract())
    }

    pub fn set_cmd(&mut self, reg: HbaPortCmdReg) {
        self.cmd.set(reg.0.get())
    }

    pub fn tfd(&self) -> LocalRegisterCopy<u32, HbaPortTfdReg::Register> {
        self.tfd.extract()
    }

    pub fn sig(&self) -> HbaPortSigReg {
        HbaPortSigReg(self.sig.extract())
    }

    pub fn ssts(&self) -> HbaPortSstsReg {
        HbaPortSstsReg(self.ssts.extract())
    }

    pub fn sctl(&self) -> HbaPortSctlReg {
        HbaPortSctlReg(self.sctl.extract())
    }

    pub fn set_sctl(&mut self, reg: HbaPortSctlReg) {
        self.sctl.set(reg.0.get())
    }

    pub fn serr(&self) -> LocalRegisterCopy<u32, HbaPortSerr::Register> {
        self.serr.extract()
    }

    pub fn set_serr(&mut self, reg: LocalRegisterCopy<u32, HbaPortSerr::Register>) {
        self.serr.set(reg.get())
    }

    pub fn sact(&self) -> u32 {
        self.sact.get()
    }

    pub fn set_sact(&mut self, v: u32) {
        self.sact.set(v)
    }

    pub fn ci(&self) -> u32 {
        self.ci.get()
    }

    pub fn set_ci(&mut self, v: u32) {
        self.ci.set(v)
    }

    pub fn sntf(&self) -> u32 {
        self.sntf.get()
    }

    pub fn set_sntf(&mut self, v: u32) {
        self.sntf.set(v)
    }

    pub fn fbs(&self) -> HbaPortFbs {
        HbaPortFbs(self.fbs.extract())
    }

    pub fn set_fbs(&mut self, reg: HbaPortFbs) {
        self.fbs.set(reg.0.get())
    }

    pub fn devslp(&self) -> HbaPortDevslp {
        HbaPortDevslp(self.devslp.extract())
    }

    pub fn set_devslp(&mut self, reg: HbaPortDevslp) {
        self.devslp.set(reg.0.get())
    }

    pub fn vendor(&self) -> &[u32] {
        unsafe { &self.vendor }
    }
}
