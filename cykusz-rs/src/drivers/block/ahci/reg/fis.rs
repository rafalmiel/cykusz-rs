use bit_field::BitField;
use mmio::VCell;

use crate::drivers::block::ahci::reg::ata::AtaCommand;

#[repr(u8)]
pub enum FisType {
    RegH2D = 0x27,
    RegD2H = 0x34,
    DmaAct = 0x39,
    DmaSetup = 0x41,
    Data = 0x46,
    Bist = 0x58,
    PioSetup = 0x5F,
    DevBits = 0xA1,
}

#[repr(C, packed)]
pub struct FisRegH2D {
    fis_type: VCell<FisType>,
    flags: VCell<u8>,
    command: VCell<AtaCommand>,
    featurel: VCell<u8>,

    lba0: VCell<u8>,
    lba1: VCell<u8>,
    lba2: VCell<u8>,
    device: VCell<u8>,

    lba3: VCell<u8>,
    lba4: VCell<u8>,
    lba5: VCell<u8>,
    featureh: VCell<u8>,

    countl: VCell<u8>,
    counth: VCell<u8>,
    icc: VCell<u8>,
    control: VCell<u8>,

    _rsv1: [u8; 4],
}

#[repr(C, packed)]
pub struct FisRegD2H {
    fis_type: VCell<FisType>,
    flags: VCell<u8>,
    status: VCell<u8>,
    error: VCell<u8>,

    lba0: VCell<u8>,
    lba1: VCell<u8>,
    lba2: VCell<u8>,
    device: VCell<u8>,

    lba3: VCell<u8>,
    lba4: VCell<u8>,
    lba5: VCell<u8>,
    _rsv1: u8,

    countl: VCell<u8>,
    counth: VCell<u8>,

    _rsv2: [u8; 6],
}

#[repr(C, packed)]
pub struct FisData {
    fis_type: VCell<FisType>,

    flags: VCell<u8>,

    _rsv: [u8; 2],

    data: [u32; 1],
}

#[repr(C, packed)]
pub struct FisPioSetup {
    fis_type: VCell<FisType>,
    flags: VCell<u8>,
    status: VCell<u8>,
    error: VCell<u8>,

    lba0: VCell<u8>,
    lba1: VCell<u8>,
    lba2: VCell<u8>,
    device: VCell<u8>,

    lba3: VCell<u8>,
    lba4: VCell<u8>,
    lba5: VCell<u8>,
    _rsv1: u8,

    countl: VCell<u8>,
    counth: VCell<u8>,

    _rsv2: u8,

    e_status: VCell<u8>,

    tc: VCell<u16>,

    _rsv3: [u8; 2],
}

#[repr(C, packed)]
pub struct FisDmaSetup {
    fis_type: VCell<FisType>,

    flags: VCell<u8>,

    _rsv1: [u8; 2],

    dma_buf_id: VCell<u64>,

    _rsv2: u32,

    dma_buf_offset: VCell<u32>,

    tx_count: VCell<u32>,

    _rsv3: u32,
}

#[repr(C, packed)]
pub struct HbaFis {
    dsfis: FisDmaSetup,
    _pad1: [u8; 4],

    psfis: FisPioSetup,
    _pad2: [u8; 12],

    rfis: FisRegD2H,
    _pad3: [u8; 4],

    sdbfis: [u8; 8],

    ufis: [u8; 64],

    _rsv1: [u8; 0x100 - 0xA0],
}
