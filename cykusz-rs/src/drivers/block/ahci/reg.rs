use mmio::VCell;

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
pub struct HbaMem {
    cap: VCell<u32>,
    ghc: VCell<u32>,
    is: VCell<u32>,
    pi: VCell<u32>,
    vs: VCell<u32>,
    ccc_ctl: VCell<u32>,
    ccc_pts: VCell<u32>,
    em_loc: VCell<u32>,
    em_ctl: VCell<u32>,
    cap2: VCell<u32>,
    bohc: VCell<u32>,
    _rsv: [u8; 0xa0 - 0x2c],
    vendor: [u8; 0x100 - 0xa0],
}

#[repr(C, packed)]
pub struct HbaPort {
    clb: VCell<u64>,
    fb: VCell<u64>,
    is: VCell<u32>,
    ie: VCell<u32>,
    cmd: VCell<u32>,
    _rsv: u32,
    tfd: VCell<u32>,
    sig: VCell<u32>,
    ssts: VCell<u32>,
    sctl: VCell<u32>,
    serr: VCell<u32>,
    sact: VCell<u32>,
    ci: VCell<u32>,
    sntf: VCell<u32>,
    fbs: VCell<u32>,
    _rsv1: [u32; 11],
    vendor: [u32; 4],
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

impl HbaMem {
    pub fn cap(&self) -> u32 {
        unsafe { self.cap.get() }
    }

    pub fn ghc(&self) -> u32 {
        unsafe { self.ghc.get() }
    }

    pub fn is(&self) -> u32 {
        unsafe { self.is.get() }
    }

    pub fn pi(&self) -> u32 {
        unsafe { self.pi.get() }
    }

    pub fn vs(&self) -> u32 {
        unsafe { self.vs.get() }
    }

    pub fn ccc_ctl(&self) -> u32 {
        unsafe { self.ccc_ctl.get() }
    }

    pub fn ccc_pts(&self) -> u32 {
        unsafe { self.ccc_pts.get() }
    }

    pub fn em_loc(&self) -> u32 {
        unsafe { self.em_loc.get() }
    }

    pub fn em_ctl(&self) -> u32 {
        unsafe { self.em_ctl.get() }
    }

    pub fn cap2(&self) -> u32 {
        unsafe { self.cap2.get() }
    }

    pub fn bohc(&self) -> u32 {
        unsafe { self.bohc.get() }
    }

    pub fn vendor(&self) -> &[u8] {
        &self.vendor
    }
}

impl HbaPort {
    fn clb(&self) -> u64 {
        unsafe { self.clb.get() }
    }

    fn fb(&self) -> u64 {
        unsafe { self.fb.get() }
    }

    fn is(&self) -> u32 {
        unsafe { self.is.get() }
    }

    fn ie(&self) -> u32 {
        unsafe { self.ie.get() }
    }

    fn cmd(&self) -> u32 {
        unsafe { self.cmd.get() }
    }

    fn tfd(&self) -> u32 {
        unsafe { self.tfd.get() }
    }

    fn sig(&self) -> u32 {
        unsafe { self.sig.get() }
    }

    fn ssts(&self) -> u32 {
        unsafe { self.ssts.get() }
    }

    fn sctl(&self) -> u32 {
        unsafe { self.sctl.get() }
    }

    fn serr(&self) -> u32 {
        unsafe { self.serr.get() }
    }

    fn sact(&self) -> u32 {
        unsafe { self.sact.get() }
    }

    fn ci(&self) -> u32 {
        unsafe { self.ci.get() }
    }

    fn sntf(&self) -> u32 {
        unsafe { self.sntf.get() }
    }

    fn fbs(&self) -> u32 {
        unsafe { self.fbs.get() }
    }

    fn vendor(&self) -> &[u32] {
        &self.vendor
    }
}
