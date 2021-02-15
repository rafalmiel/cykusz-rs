use bit_field::BitField;
use mmio::VCell;

use crate::drivers::block::ata::AtaCommand;
use crate::kernel::mm::PhysAddr;
use crate::kernel::utils::slice::ToBytesMut;

#[repr(u8)]
#[derive(Copy, Clone)]
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

    count: VCell<u16>,
    icc: VCell<u8>,
    control: VCell<u8>,

    _rsv1: [u8; 4],
}

impl FisRegH2D {
    pub fn reset(&mut self) {
        let mut this = self;
        this.to_bytes_mut().fill(0);
    }

    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn c(&self) -> bool {
        unsafe { self.flags.get().get_bit(15) }
    }

    pub fn set_c(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(7, i));
        }
    }

    pub fn command(&self) -> AtaCommand {
        unsafe { self.command.get() }
    }

    pub fn set_command(&mut self, cmd: AtaCommand) {
        unsafe {
            self.command.set(cmd);
        }
    }

    pub fn featurel(&self) -> usize {
        unsafe { self.featurel.get() as usize }
    }

    pub fn set_featurel(&mut self, v: u8) {
        unsafe {
            self.featurel.set(v);
        }
    }

    pub fn featureh(&self) -> usize {
        unsafe { self.featureh.get() as usize }
    }

    pub fn set_featureh(&mut self, v: u8) {
        unsafe {
            self.featureh.set(v);
        }
    }

    pub fn device(&self) -> usize {
        unsafe { self.device.get() as usize }
    }

    pub fn set_device(&mut self, d: u8) {
        unsafe {
            self.device.set(d);
        }
    }

    pub fn control(&self) -> usize {
        unsafe { self.control.get() as usize }
    }

    pub fn set_control(&mut self, d: u8) {
        unsafe {
            self.control.set(d);
        }
    }

    pub fn icc(&self) -> usize {
        unsafe { self.icc.get() as usize }
    }

    pub fn set_icc(&mut self, d: u8) {
        unsafe {
            self.icc.set(d);
        }
    }

    pub fn count(&self) -> usize {
        unsafe { self.count.get() as usize }
    }

    pub fn set_count(&mut self, d: u16) {
        unsafe {
            self.count.set(d);
        }
    }

    pub fn set_lba(&mut self, addr: usize) {
        self.set_lba0(addr as u8);
        self.set_lba1((addr >> 8) as u8);
        self.set_lba2((addr >> 16) as u8);
        self.set_lba3((addr >> 24) as u8);
        self.set_lba4((addr >> 32) as u8);
        self.set_lba5((addr >> 40) as u8);
    }

    pub fn lba0(&self) -> usize {
        unsafe { self.lba0.get() as usize }
    }

    pub fn set_lba0(&mut self, v: u8) {
        unsafe {
            self.lba0.set(v);
        }
    }

    pub fn lba1(&self) -> usize {
        unsafe { self.lba1.get() as usize }
    }

    pub fn set_lba1(&mut self, v: u8) {
        unsafe {
            self.lba1.set(v);
        }
    }

    pub fn lba2(&self) -> usize {
        unsafe { self.lba2.get() as usize }
    }

    pub fn set_lba2(&mut self, v: u8) {
        unsafe {
            self.lba2.set(v);
        }
    }

    pub fn lba3(&self) -> usize {
        unsafe { self.lba3.get() as usize }
    }

    pub fn set_lba3(&mut self, v: u8) {
        unsafe {
            self.lba3.set(v);
        }
    }

    pub fn lba4(&self) -> usize {
        unsafe { self.lba4.get() as usize }
    }

    pub fn set_lba4(&mut self, v: u8) {
        unsafe {
            self.lba4.set(v);
        }
    }

    pub fn lba5(&self) -> usize {
        unsafe { self.lba5.get() as usize }
    }

    pub fn set_lba5(&mut self, v: u8) {
        unsafe {
            self.lba5.set(v);
        }
    }
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

    count: VCell<u16>,

    _rsv2: [u8; 6],
}

impl FisRegD2H {
    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn i(self) -> bool {
        unsafe { self.flags.get().get_bit(14) }
    }

    pub fn set_i(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(14, i));
        }
    }

    pub fn status(&self) -> u8 {
        unsafe { self.status.get() }
    }

    pub fn set_status(&mut self, s: u8) {
        unsafe {
            self.status.set(s);
        }
    }

    pub fn error(&self) -> u8 {
        unsafe { self.error.get() }
    }

    pub fn set_error(&mut self, e: u8) {
        unsafe {
            self.error.set(e);
        }
    }

    pub fn device(&self) -> usize {
        unsafe { self.device.get() as usize }
    }

    pub fn set_device(&mut self, d: u8) {
        unsafe {
            self.device.set(d);
        }
    }

    pub fn count(&self) -> usize {
        unsafe { self.count.get() as usize }
    }

    pub fn set_count(&mut self, d: u16) {
        unsafe {
            self.count.set(d);
        }
    }

    pub fn lba0(&self) -> usize {
        unsafe { self.lba0.get() as usize }
    }

    pub fn set_lba0(&mut self, v: u8) {
        unsafe {
            self.lba0.set(v);
        }
    }

    pub fn lba1(&self) -> usize {
        unsafe { self.lba1.get() as usize }
    }

    pub fn set_lba1(&mut self, v: u8) {
        unsafe {
            self.lba1.set(v);
        }
    }

    pub fn lba2(&self) -> usize {
        unsafe { self.lba2.get() as usize }
    }

    pub fn set_lba2(&mut self, v: u8) {
        unsafe {
            self.lba2.set(v);
        }
    }

    pub fn lba3(&self) -> usize {
        unsafe { self.lba3.get() as usize }
    }

    pub fn set_lba3(&mut self, v: u8) {
        unsafe {
            self.lba3.set(v);
        }
    }

    pub fn lba4(&self) -> usize {
        unsafe { self.lba4.get() as usize }
    }

    pub fn set_lba4(&mut self, v: u8) {
        unsafe {
            self.lba4.set(v);
        }
    }

    pub fn lba5(&self) -> usize {
        unsafe { self.lba5.get() as usize }
    }

    pub fn set_lba5(&mut self, v: u8) {
        unsafe {
            self.lba5.set(v);
        }
    }
}

#[repr(C, packed)]
pub struct FisData {
    fis_type: VCell<FisType>,

    flags: VCell<u8>,

    _rsv: [u8; 2],

    data: [u32; 1],
}

impl FisData {
    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn data(&self, dwords: usize) -> &[u32] {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr(), dwords) }
    }

    fn data_mut(&mut self, dwords: usize) -> &mut [u32] {
        unsafe { core::slice::from_raw_parts_mut(self.data.as_mut_ptr(), dwords) }
    }

    pub fn set_data(&mut self, data: &[u32]) {
        self.data_mut(data.len()).copy_from_slice(data);
    }
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

    count: VCell<u16>,

    _rsv2: u8,

    e_status: VCell<u8>,

    tc: VCell<u16>,

    _rsv3: [u8; 2],
}

impl FisPioSetup {
    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn i(self) -> bool {
        unsafe { self.flags.get().get_bit(14) }
    }

    pub fn set_i(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(14, i));
        }
    }

    pub fn status(&self) -> u8 {
        unsafe { self.status.get() }
    }

    pub fn set_status(&mut self, s: u8) {
        unsafe {
            self.status.set(s);
        }
    }

    pub fn error(&self) -> u8 {
        unsafe { self.error.get() }
    }

    pub fn set_error(&mut self, e: u8) {
        unsafe {
            self.error.set(e);
        }
    }

    pub fn device(&self) -> usize {
        unsafe { self.device.get() as usize }
    }

    pub fn set_device(&mut self, d: u8) {
        unsafe {
            self.device.set(d);
        }
    }

    pub fn count(&self) -> usize {
        unsafe { self.count.get() as usize }
    }

    pub fn set_count(&mut self, d: u16) {
        unsafe {
            self.count.set(d);
        }
    }

    pub fn lba0(&self) -> usize {
        unsafe { self.lba0.get() as usize }
    }

    pub fn set_lba0(&mut self, v: u8) {
        unsafe {
            self.lba0.set(v);
        }
    }

    pub fn lba1(&self) -> usize {
        unsafe { self.lba1.get() as usize }
    }

    pub fn set_lba1(&mut self, v: u8) {
        unsafe {
            self.lba1.set(v);
        }
    }

    pub fn lba2(&self) -> usize {
        unsafe { self.lba2.get() as usize }
    }

    pub fn set_lba2(&mut self, v: u8) {
        unsafe {
            self.lba2.set(v);
        }
    }

    pub fn lba3(&self) -> usize {
        unsafe { self.lba3.get() as usize }
    }

    pub fn set_lba3(&mut self, v: u8) {
        unsafe {
            self.lba3.set(v);
        }
    }

    pub fn lba4(&self) -> usize {
        unsafe { self.lba4.get() as usize }
    }

    pub fn set_lba4(&mut self, v: u8) {
        unsafe {
            self.lba4.set(v);
        }
    }

    pub fn lba5(&self) -> usize {
        unsafe { self.lba5.get() as usize }
    }

    pub fn set_lba5(&mut self, v: u8) {
        unsafe {
            self.lba5.set(v);
        }
    }

    pub fn e_status(&self) -> usize {
        unsafe { self.e_status.get() as usize }
    }

    pub fn set_e_status(&mut self, s: u8) {
        unsafe {
            self.e_status.set(s);
        }
    }

    pub fn transfer_count(&self) -> usize {
        unsafe { self.tc.get() as usize }
    }

    pub fn set_transfer_count(&mut self, c: u16) {
        unsafe {
            self.tc.set(c);
        }
    }
}

#[repr(C, packed)]
pub struct FisSetDeviceBits {
    fis_type: VCell<FisType>,

    flags: VCell<u8>,
    status: VCell<u8>,
    error: VCell<u8>,

    protocol_specific: VCell<u32>,
}

impl FisSetDeviceBits {
    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn i(self) -> bool {
        unsafe { self.flags.get().get_bit(14) }
    }

    pub fn set_i(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(14, i));
        }
    }

    pub fn n(self) -> bool {
        unsafe { self.flags.get().get_bit(15) }
    }

    pub fn set_n(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(15, i));
        }
    }

    pub fn status(&self) -> u8 {
        unsafe { self.status.get() }
    }

    pub fn set_status(&mut self, s: u8) {
        unsafe {
            self.status.set(s);
        }
    }

    pub fn error(&self) -> u8 {
        unsafe { self.error.get() }
    }

    pub fn set_error(&mut self, e: u8) {
        unsafe {
            self.error.set(e);
        }
    }
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

impl FisDmaSetup {
    pub fn fis_type(&self) -> FisType {
        unsafe { self.fis_type.get() }
    }

    pub fn set_fis_type(&mut self, t: FisType) {
        unsafe {
            self.fis_type.set(t);
        }
    }

    pub fn pm_port(&self) -> usize {
        unsafe { self.flags.get().get_bits(8..=12) as usize }
    }

    pub fn set_pm_port(&mut self, port: usize) {
        unsafe {
            self.flags
                .set(*self.flags.get().set_bits(8..=12, port as u8));
        }
    }

    pub fn d(self) -> bool {
        unsafe { self.flags.get().get_bit(13) }
    }

    pub fn set_d(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(13, i));
        }
    }

    pub fn i(self) -> bool {
        unsafe { self.flags.get().get_bit(14) }
    }

    pub fn set_i(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(14, i));
        }
    }

    pub fn a(self) -> bool {
        unsafe { self.flags.get().get_bit(15) }
    }

    pub fn set_a(&mut self, i: bool) {
        unsafe {
            self.flags.set(*self.flags.get().set_bit(15, i));
        }
    }

    pub fn buf_id(&self) -> u64 {
        unsafe { self.dma_buf_id.get() }
    }

    pub fn set_buf_id(&mut self, id: u64) {
        unsafe {
            self.dma_buf_id.set(id);
        }
    }

    pub fn buf_offset(&self) -> usize {
        unsafe { self.dma_buf_offset.get() as usize }
    }

    pub fn set_buf_offset(&mut self, off: u32) {
        unsafe {
            self.dma_buf_offset.set(off);
        }
    }

    pub fn tx_count(&self) -> usize {
        unsafe { self.tx_count.get() as usize }
    }

    pub fn set_tx_count(&mut self, c: u32) {
        unsafe {
            self.tx_count.set(c);
        }
    }
}

#[repr(C, packed)]
pub struct HbaFis {
    dma_setup: FisDmaSetup,
    _pad1: [u8; 4],

    pio_setup: FisPioSetup,
    _pad2: [u8; 12],

    d2h: FisRegD2H,
    _pad3: [u8; 4],

    dev_set_bits: FisSetDeviceBits,

    unknown: [u8; 64],

    _rsv1: [u8; 0x100 - 0xA0],
}

impl HbaFis {
    pub fn dma_setup(&self) -> &FisDmaSetup {
        &self.dma_setup
    }

    pub fn pio_setup(&self) -> &FisPioSetup {
        &self.pio_setup
    }

    pub fn d2h(&self) -> &FisRegD2H {
        &self.d2h
    }

    pub fn dev_set_bits(&self) -> &FisSetDeviceBits {
        &self.dev_set_bits
    }

    pub fn unknown(&self) -> [u8; 64] {
        self.unknown
    }
}

impl HbaFis {}
