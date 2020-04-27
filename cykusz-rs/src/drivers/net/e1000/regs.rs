#[repr(u16)]
pub enum Regs {
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

    Rdtr = 0x2820,
    // RX Delay Timer Register
    RxDCtrl = 0x3828,
    // RX Descriptor Control
    Radv = 0x282C,
    // RX Int. Absolute Delay Timer
    Rsrpd = 0x2C00, // RX Small Packet Detect Interrupt

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

bitflags! {
    pub struct IntFlags: u32 {
        const TXDW          = (1 << 0);     // Transmit Descriptor Written Back
        const TXQE          = (1 << 1);     // Transmit Queue Empty
        const LSC           = (1 << 2);     // Link Status Change
        const RXDMT0        = (1 << 4);     // Receive Descriptor Minimum Threshold
        const DSW           = (1 << 5);     // Disable SW Write Access
        const RXO           = (1 << 6);     // Receiver Overrun
        const RXT0          = (1 << 7);     // Receiver Timer Interrupt
        const MDAC          = (1 << 9);     // MDIO Access Complete
        const PHYINT        = (1 << 12);    // PHY Interrupt
        const LSECPN        = (1 << 14);    // MACsec Packet Number
        const TXD_LOW       = (1 << 15);    // Transmit Descriptor Low Threshold hit
        const SRPD          = (1 << 16);    // Small Receive Packet Detected
        const ACK           = (1 << 17);    // Receive ACK Frame Detected
        const ECCER         = (1 << 22);    // ECC Error
    }
}

impl Default for TStatus {
    fn default() -> Self {
        TStatus { bits: 0 }
    }
}

impl TCtl {
    pub fn set_collision_threshold(&mut self, val: u8) {
        self.bits |= (val as u32) << 4;
    }

    pub fn set_collision_distance(&mut self, val: u8) {
        self.bits |= (val as u32) << 12;
    }
}

impl Default for TCtl {
    fn default() -> Self {
        TCtl { bits: 1u32 << 28 }
    }
}

impl Default for IntFlags {
    fn default() -> Self {
        IntFlags::TXDW
            | IntFlags::TXQE
            | IntFlags::LSC
            | IntFlags::RXDMT0
            | IntFlags::DSW
            | IntFlags::RXO
            | IntFlags::RXT0
            | IntFlags::MDAC
            | IntFlags::PHYINT
            | IntFlags::LSECPN
            | IntFlags::TXD_LOW
            | IntFlags::SRPD
            | IntFlags::ACK
            | IntFlags::ECCER
    }
}
