#![allow(unused)]

mod mem;
mod port;

use bit_field::BitField;
use mmio::VCell;

pub use self::mem::*;
pub use self::port::*;

use crate::kernel::mm::VirtAddr;

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
    command: VCell<u8>,
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

#[repr(C, packed)]
pub struct HbaCmdHeader {
    cmd_flags: VCell<u8>,
    flags: VCell<u8>,

    prdtl: VCell<u16>,
    prdbc: VCell<u32>,

    ctba: VCell<u32>,
    ctbau: VCell<u32>,

    _rsv1: [u8; 4],
}

#[repr(C, packed)]
pub struct HbaCmdTbl {
    cfis: [u8; 64],

    acmd: [u8; 16],

    _rsv1: [u8; 48],

    prdt_entry: [HbaPrdtEntry; 1],
}

#[repr(C, packed)]
pub struct HbaPrdtEntry {
    dba: VCell<u32>,
    dbau: VCell<u32>,

    _rsv1: u32,

    flags: VCell<u32>,
}
