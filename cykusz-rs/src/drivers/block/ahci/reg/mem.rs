use super::HbaPort;
use bit_field::BitField;
use tock_registers::fields::FieldValue;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::ReadWrite;
use tock_registers::{register_bitfields, register_structs, LocalRegisterCopy};

register_bitfields! [
    u32,

    pub HbaMemCapRegDef [
        NUM_PORTS OFFSET(0) NUMBITS(5) [],
        SXS OFFSET(5) NUMBITS(1) [], // Supports External SATA
        EMS OFFSET(6) NUMBITS(1) [],  // Enclosure Management Supported
        CCCS OFFSET(7) NUMBITS(1) [], // Command Completion Coalescing Supported
        NUM_CMD_PORTS OFFSET(8) NUMBITS(5) [],
        PSC OFFSET(13) NUMBITS(1) [], // Partial State Capable
        SSC OFFSET(14) NUMBITS(1) [], // Slumber State Capable
        PMD OFFSET(15) NUMBITS(1) [], // PIO Multiple DRQ Block
        FBSS OFFSET(16) NUMBITS(1) [], // FIS-based Switching Supported
        SPM OFFSET(17) NUMBITS(1) [], // Supports Port Multiplier
        SAM OFFSET(18) NUMBITS(1) [], // Supports AHCI mode only
        ISPEED OFFSET(20) NUMBITS(4) [
             Gen1 = 0x0001,
             Gen2 = 0x0010,
             Gen3 = 0x0011,
        ],
        SCLO OFFSET(24) NUMBITS(1) [], // Supports Command List Override
        SAL OFFSET(25) NUMBITS(1) [], // Supports Activity LED
        SALP OFFSET(26) NUMBITS(1) [], // Supports Aggressive Link Power Mgmt
        SSS OFFSET(27) NUMBITS(1) [], // Supports Staggered Spin-up
        SMPS OFFSET(28) NUMBITS(1) [], // Supports Mechanical Presence Switch
        SSNTF OFFSET(29) NUMBITS(1) [], // Supports SNotification Register
        SNCQ OFFSET(30) NUMBITS(1) [], // Supports Native Command Queuing
        S64A OFFSET(31) NUMBITS(1) [], // Supports 64-bit Addressing
    ]
];

pub struct HbaMemCapReg(LocalRegisterCopy<u32, HbaMemCapRegDef::Register>);

impl HbaMemCapReg {
    pub fn num_ports(&self) -> usize {
        self.0.read(HbaMemCapRegDef::NUM_PORTS) as usize
    }

    pub fn num_cmd_ports(&self) -> usize {
        self.0.read(HbaMemCapRegDef::NUM_CMD_PORTS) as usize
    }

    pub fn ispeed(&self) -> HbaMemCapRegDef::ISPEED::Value {
        self.0
            .read_as_enum(HbaMemCapRegDef::ISPEED)
            .expect("Invalid ISPEED")
    }
}

register_bitfields! [
    u32,

    pub HbaMemGhcReg [
        HR 0,
        IE 1,
        MRSM 2,
        AE 31,
    ]
];

register_bitfields! [
    u32,

    HbaMemCCCCtlRegDef [
        ENABLE OFFSET(0) NUMBITS(1),
        INTERRUPT OFFSET(3) NUMBITS(5),
        COMMAND_COMPLETION OFFSET(8) NUMBITS(8),
        TIMEOUT_VALUE OFFSET(16) NUMBITS(16),
    ]
];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaMemCCCCtlReg(LocalRegisterCopy<u32, HbaMemCCCCtlRegDef::Register>);

impl HbaMemCCCCtlReg {
    pub fn enable(&self) -> bool {
        self.0.is_set(HbaMemCCCCtlRegDef::ENABLE)
    }

    pub fn set_enable(&mut self, e: bool) -> HbaMemCCCCtlReg {
        self.0.modify(if e {
            HbaMemCCCCtlRegDef::ENABLE::SET
        } else {
            HbaMemCCCCtlRegDef::ENABLE::CLEAR
        });

        *self
    }

    pub fn interrupt(&self) -> usize {
        self.0.read(HbaMemCCCCtlRegDef::INTERRUPT) as usize
    }

    pub fn command_completions(&self) -> usize {
        self.0.read(HbaMemCCCCtlRegDef::COMMAND_COMPLETION) as usize
    }

    pub fn set_command_completions(&mut self, cmds: usize) -> HbaMemCCCCtlReg {
        self.0
            .modify(HbaMemCCCCtlRegDef::COMMAND_COMPLETION.val(cmds as u32));
        *self
    }

    pub fn timeout_value(&self) -> usize {
        self.0.read(HbaMemCCCCtlRegDef::TIMEOUT_VALUE) as usize
    }

    pub fn set_timeout_value(&mut self, timeout: usize) -> HbaMemCCCCtlReg {
        self.0
            .modify(HbaMemCCCCtlRegDef::TIMEOUT_VALUE.val(timeout as u32));
        *self
    }
}

register_bitfields! [
    u32,

    HbaMemEmLocRegDef [
        BUFFER_SIZE OFFSET(0) NUMBITS(16),
        OFFSET OFFSET(16) NUMBITS(16),
    ]
];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HbaMemEmLocReg(LocalRegisterCopy<u32, HbaMemEmLocRegDef::Register>);

impl HbaMemEmLocReg {
    pub fn buffer_size(&self) -> usize {
        self.0.read(HbaMemEmLocRegDef::BUFFER_SIZE) as usize
    }

    pub fn offset(&self) -> usize {
        self.0.read(HbaMemEmLocRegDef::OFFSET) as usize
    }
}

register_bitfields! [
    u32,

    HbaMemEmlCtlReg [
        STS_MR     0,  // Message Received
        CTL_TM     8,  // Transmit Message
        CTL_RST    9,  // Reset
        SUPP_LED   16, // LED Message Types
        SUPP_SAFTE 17, // SAF-TE Enclosure Management Messages
        SUPP_SES2  18, // SES-2 Enclosure Management Messages
        SUPP_SGPIO 19, // SGPIO Enclosure Management Messages
        ATTR_SMB   24, // Single Message Buffer
        ATTR_XMT   25, // Transmit Only
        ATTR_ALHD  26, // Activity LED Hardware Driven
        ATTR_PM    27, // Port Multiplier Support
    ]
];

register_bitfields! [
    u32,

    HbaMemCap2Reg [
        BOH  0, // BIOS/OS Handoff
        NVMP 1, // NVMHCI Present
        APST 2, // Automatic Partial to Slumber Transitions
        SDS  3, // Supports Device Sleep
        SADM 4, // Supports Aggressive Device Sleep management
        DESO 5, // DevSleep Entrance from Slumber Only
    ]
];

register_bitfields! [
    u32,

    pub HbaMemBohcReg [
        BOS  0, // BIOS Owned Semaphore
        OOS  1, // OS Owned Semaphore
        SOOE 2, // SMI on OS Ownership Change Enable
        OOC  3, // OS Ownership Change
        BB   4, // BIOS Busy
    ]
];

register_structs! {
    pub HbaMem {
        (0x0000 => cap: ReadWrite<u32, HbaMemCapRegDef::Register>),
        (0x0004 => ghc: ReadWrite<u32, HbaMemGhcReg::Register>),
        (0x0008 => is: ReadWrite<u32>),
        (0x000C => pi: ReadWrite<u32>),
        (0x0010 => vs: ReadWrite<u32>),
        (0x0014 => ccc_ctl: ReadWrite<u32, HbaMemCCCCtlRegDef::Register>),
        (0x0018 => ccc_pts: ReadWrite<u32>),
        (0x001C => em_loc: ReadWrite<u32, HbaMemEmLocRegDef::Register>),
        (0x0020 => em_ctl: ReadWrite<u32, HbaMemEmlCtlReg::Register>),
        (0x0024 => cap2: ReadWrite<u32, HbaMemCap2Reg::Register>),
        (0x0028 => bohc: ReadWrite<u32, HbaMemBohcReg::Register>),
        (0x002C => _rsv),
        (0x00A0 => vendor: [u8; 0x60]),
        (0x0100 => @END),
    }
}

impl HbaMem {
    pub fn cap(&self) -> HbaMemCapReg {
        HbaMemCapReg(self.cap.extract())
    }

    pub fn ghc(&self) -> LocalRegisterCopy<u32, HbaMemGhcReg::Register> {
        self.ghc.extract()
    }

    pub fn modify_ghc(&mut self, ghc: FieldValue<u32, HbaMemGhcReg::Register>) {
        self.ghc.modify(ghc);
    }

    pub fn is(&self) -> u32 {
        self.is.get()
    }

    pub fn set_is(&mut self, is: u32) {
        self.is.set(is)
    }

    pub fn pi(&self) -> u32 {
        self.pi.get()
    }

    pub fn vs(&self) -> u32 {
        self.vs.get()
    }

    pub fn ccc_ctl(&self) -> HbaMemCCCCtlReg {
        HbaMemCCCCtlReg(self.ccc_ctl.extract())
    }

    pub fn set_ccc_ctl(&mut self, ccc: LocalRegisterCopy<u32, HbaMemCCCCtlRegDef::Register>) {
        self.ccc_ctl.set(ccc.get());
    }

    pub fn ccc_pts(&self) -> u32 {
        self.ccc_pts.get()
    }

    pub fn set_ccc_pts(&mut self, ports: u32) {
        self.ccc_pts.set(ports)
    }

    pub fn em_loc(&self) -> HbaMemEmLocReg {
        HbaMemEmLocReg(self.em_loc.extract())
    }

    pub fn em_ctl(&self) -> LocalRegisterCopy<u32, HbaMemEmlCtlReg::Register> {
        self.em_ctl.extract()
    }

    pub fn set_em_ctl(&mut self, reg: FieldValue<u32, HbaMemEmlCtlReg::Register>) {
        self.em_ctl.write(reg);
    }

    pub fn cap2(&self) -> LocalRegisterCopy<u32, HbaMemCap2Reg::Register> {
        self.cap2.extract()
    }

    pub fn bohc(&self) -> LocalRegisterCopy<u32, HbaMemBohcReg::Register> {
        self.bohc.extract()
    }

    pub fn set_bohc(&mut self, reg: FieldValue<u32, HbaMemBohcReg::Register>) {
        self.bohc.write(reg);
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
