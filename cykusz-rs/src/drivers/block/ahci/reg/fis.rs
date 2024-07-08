use crate::drivers::block::ahci::reg::fis;
use crate::drivers::block::ata::AtaCommand;
use bit_field::BitField;
use tock_registers::fields::FieldValue;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::ReadWrite;
use tock_registers::{register_bitfields, register_structs, RegisterLongName, UIntLike};

register_bitfields! [
    u8,

    pub FisType [
        TYPE OFFSET(0) NUMBITS(8) [
            RegH2D = 0x27,
            RegD2H = 0x34,
            DmaAct = 0x39,
            DmaSetup = 0x41,
            Data = 0x46,
            Bist = 0x58,
            PioSetup = 0x5F,
            DevBits = 0xA1,
        ]
    ],

    pub FisFlags [
        PM_PORT OFFSET(0) NUMBITS(4),
        D OFFSET(5) NUMBITS(1),
        I OFFSET(6) NUMBITS(1),
        C OFFSET(7) NUMBITS(1),
        N OFFSET(7) NUMBITS(1),
        A OFFSET(7) NUMBITS(1),
    ]
];

register_structs! {
    pub FisRegH2D {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => command: ReadWrite<u8>),
        (0x0003 => featurel: ReadWrite<u8>),
        (0x0004 => lba0: ReadWrite<u8>),
        (0x0005 => lba1: ReadWrite<u8>),
        (0x0006 => lba2: ReadWrite<u8>),
        (0x0007 => device: ReadWrite<u8>),
        (0x0008 => lba3: ReadWrite<u8>),
        (0x0009 => lba4: ReadWrite<u8>),
        (0x000A => lba5: ReadWrite<u8>),
        (0x000B => featureh: ReadWrite<u8>),
        (0x000C => count: [ReadWrite<u8>; 2]),
        (0x000E => icc: ReadWrite<u8>),
        (0x000F => control: ReadWrite<u8>),
        (0x0010 => _reserved),
        (0x0014 => @END),
    }
}

impl FisRegH2D {
    pub fn reset(&mut self) {
        self.set_fis_type(FisType::TYPE::RegH2D);
        self.set_flags(0);
        self.set_featurel(0);
        self.set_featureh(0);
        self.set_lba(0, 0);
        self.set_count(0);
        self.set_icc(0);
        self.set_control(0);
        self._reserved.fill(0);
    }

    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        unsafe {
            self.fis_type.write(t);
        }
    }

    pub fn flags(&self) -> u8 {
        self.flags.get()
    }

    pub fn set_flags(&mut self, v: u8) {
        self.flags.set(v);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
    }

    pub fn c(&self) -> bool {
        self.flags.is_set(FisFlags::C)
    }

    pub fn set_c(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::C::SET
        } else {
            FisFlags::C::CLEAR
        });
    }

    pub fn set_command(&mut self, cmd: AtaCommand) {
        self.command.set(cmd as u8);
    }

    pub fn featurel(&self) -> usize {
        self.featurel.get() as usize
    }

    pub fn set_featurel(&mut self, v: u8) {
        self.featurel.set(v);
    }

    pub fn featureh(&self) -> usize {
        self.featureh.get() as usize
    }

    pub fn set_featureh(&mut self, v: u8) {
        self.featureh.set(v);
    }

    pub fn device(&self) -> usize {
        self.device.get() as usize
    }

    pub fn set_device(&mut self, d: u8) {
        self.device.set(d);
    }

    pub fn control(&self) -> usize {
        self.control.get() as usize
    }

    pub fn set_control(&mut self, d: u8) {
        self.control.set(d);
    }

    pub fn icc(&self) -> usize {
        self.icc.get() as usize
    }

    pub fn set_icc(&mut self, d: u8) {
        self.icc.set(d);
    }

    pub fn count(&self) -> usize {
        self.count[0].get() as usize | (self.count[1].get() as usize) << 8
    }

    pub fn set_count(&mut self, d: u16) {
        self.count[0].set(d as u8);
        self.count[1].set((d >> 8) as u8);
    }

    pub fn set_lba(&mut self, addr: usize, device: u8) {
        self.set_lba0(addr as u8);
        self.set_lba1((addr >> 8) as u8);
        self.set_lba2((addr >> 16) as u8);
        self.set_device(device);
        self.set_lba3((addr >> 24) as u8);
        self.set_lba4((addr >> 32) as u8);
        self.set_lba5((addr >> 40) as u8);
    }

    pub fn lba0(&self) -> usize {
        self.lba0.get() as usize
    }

    pub fn set_lba0(&mut self, v: u8) {
        self.lba0.set(v);
    }

    pub fn lba1(&self) -> usize {
        self.lba1.get() as usize
    }

    pub fn set_lba1(&mut self, v: u8) {
        self.lba1.set(v);
    }

    pub fn lba2(&self) -> usize {
        self.lba2.get() as usize
    }

    pub fn set_lba2(&mut self, v: u8) {
        self.lba2.set(v);
    }

    pub fn lba3(&self) -> usize {
        self.lba3.get() as usize
    }

    pub fn set_lba3(&mut self, v: u8) {
        self.lba3.set(v);
    }

    pub fn lba4(&self) -> usize {
        self.lba4.get() as usize
    }

    pub fn set_lba4(&mut self, v: u8) {
        self.lba4.set(v);
    }

    pub fn lba5(&self) -> usize {
        self.lba5.get() as usize
    }

    pub fn set_lba5(&mut self, v: u8) {
        self.lba5.set(v);
    }
}

register_structs! {
    pub FisRegD2H {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => status: ReadWrite<u8>),
        (0x0003 => error: ReadWrite<u8>),
        (0x0004 => lba0: ReadWrite<u8>),
        (0x0005 => lba1: ReadWrite<u8>),
        (0x0006 => lba2: ReadWrite<u8>),
        (0x0007 => device: ReadWrite<u8>),
        (0x0008 => lba3: ReadWrite<u8>),
        (0x0009 => lba4: ReadWrite<u8>),
        (0x000A => lba5: ReadWrite<u8>),
        (0x000B => _rsv1),
        (0x000C => count: ReadWrite<u16>),
        (0x000E => _rsv2),
        (0x0014 => @END),
    }
}

impl FisRegD2H {
    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        self.fis_type.write(t);
    }

    pub fn flags(&self) -> u8 {
        self.flags.get()
    }

    pub fn set_flags(&mut self, v: u8) {
        self.flags.set(v);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
    }

    pub fn i(&self) -> bool {
        self.flags.is_set(FisFlags::I)
    }

    pub fn set_i(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::I::SET
        } else {
            FisFlags::I::CLEAR
        });
    }

    pub fn status(&self) -> u8 {
        self.status.get()
    }

    pub fn set_status(&mut self, s: u8) {
        self.status.set(s);
    }

    pub fn error(&self) -> u8 {
        self.error.get()
    }

    pub fn set_error(&mut self, e: u8) {
        self.error.set(e);
    }

    pub fn device(&self) -> usize {
        self.device.get() as usize
    }

    pub fn set_device(&mut self, d: u8) {
        self.device.set(d);
    }

    pub fn count(&self) -> usize {
        self.count.get() as usize
    }

    pub fn set_count(&mut self, d: u16) {
        self.count.set(d);
    }

    pub fn lba0(&self) -> usize {
        self.lba0.get() as usize
    }

    pub fn set_lba0(&mut self, v: u8) {
        self.lba0.set(v);
    }

    pub fn lba1(&self) -> usize {
        self.lba1.get() as usize
    }

    pub fn set_lba1(&mut self, v: u8) {
        self.lba1.set(v);
    }

    pub fn lba2(&self) -> usize {
        self.lba2.get() as usize
    }

    pub fn set_lba2(&mut self, v: u8) {
        self.lba2.set(v);
    }

    pub fn lba3(&self) -> usize {
        self.lba3.get() as usize
    }

    pub fn set_lba3(&mut self, v: u8) {
        self.lba3.set(v);
    }

    pub fn lba4(&self) -> usize {
        self.lba4.get() as usize
    }

    pub fn set_lba4(&mut self, v: u8) {
        self.lba4.set(v);
    }

    pub fn lba5(&self) -> usize {
        self.lba5.get() as usize
    }

    pub fn set_lba5(&mut self, v: u8) {
        self.lba5.set(v);
    }
}

register_structs! {
    pub FisData {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => _rsv),
        (0x0004 => data: [u32; 1]),
        (0x0008 => @END),
    }
}

impl FisData {
    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        self.fis_type.write(t);
    }

    pub fn flags(&self) -> u8 {
        self.flags.get()
    }

    pub fn set_flags(&mut self, v: u8) {
        self.flags.set(v);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
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

register_structs! {
    pub FisPioSetup {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => status: ReadWrite<u8>),
        (0x0003 => error: ReadWrite<u8>),
        (0x0004 => lba0: ReadWrite<u8>),
        (0x0005 => lba1: ReadWrite<u8>),
        (0x0006 => lba2: ReadWrite<u8>),
        (0x0007 => device: ReadWrite<u8>),
        (0x0008 => lba3: ReadWrite<u8>),
        (0x0009 => lba4: ReadWrite<u8>),
        (0x000A => lba5: ReadWrite<u8>),
        (0x000B => _rsv1),
        (0x000C => count: ReadWrite<u16>),
        (0x000E => _rsv2),
        (0x000F => e_status: ReadWrite<u8>),
        (0x0010 => tc: ReadWrite<u16>),
        (0x0012 => _rsv3),
        (0x0014 => @END),
    }
}

impl FisPioSetup {
    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        self.fis_type.write(t);
    }

    pub fn flags(&self) -> u8 {
        self.flags.get()
    }

    pub fn set_flags(&mut self, v: u8) {
        self.flags.set(v);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
    }

    pub fn i(&self) -> bool {
        self.flags.is_set(FisFlags::I)
    }

    pub fn set_i(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::I::SET
        } else {
            FisFlags::I::CLEAR
        });
    }

    pub fn status(&self) -> u8 {
        self.status.get()
    }

    pub fn set_status(&mut self, s: u8) {
        self.status.set(s);
    }

    pub fn error(&self) -> u8 {
        self.error.get()
    }

    pub fn set_error(&mut self, e: u8) {
        self.error.set(e);
    }

    pub fn device(&self) -> usize {
        self.device.get() as usize
    }

    pub fn set_device(&mut self, d: u8) {
        self.device.set(d);
    }

    pub fn count(&self) -> usize {
        self.count.get() as usize
    }

    pub fn set_count(&mut self, d: u16) {
        self.count.set(d);
    }

    pub fn lba0(&self) -> usize {
        self.lba0.get() as usize
    }

    pub fn set_lba0(&mut self, v: u8) {
        self.lba0.set(v);
    }

    pub fn lba1(&self) -> usize {
        self.lba1.get() as usize
    }

    pub fn set_lba1(&mut self, v: u8) {
        self.lba1.set(v);
    }

    pub fn lba2(&self) -> usize {
        self.lba2.get() as usize
    }

    pub fn set_lba2(&mut self, v: u8) {
        self.lba2.set(v);
    }

    pub fn lba3(&self) -> usize {
        self.lba3.get() as usize
    }

    pub fn set_lba3(&mut self, v: u8) {
        self.lba3.set(v);
    }

    pub fn lba4(&self) -> usize {
        self.lba4.get() as usize
    }

    pub fn set_lba4(&mut self, v: u8) {
        self.lba4.set(v);
    }

    pub fn lba5(&self) -> usize {
        self.lba5.get() as usize
    }

    pub fn set_lba5(&mut self, v: u8) {
        self.lba5.set(v);
    }

    pub fn e_status(&self) -> usize {
        self.e_status.get() as usize
    }

    pub fn set_e_status(&mut self, s: u8) {
        self.e_status.set(s);
    }

    pub fn transfer_count(&self) -> usize {
        self.tc.get() as usize
    }

    pub fn set_transfer_count(&mut self, c: u16) {
        self.tc.set(c);
    }
}

register_structs! {
    pub FisSetDeviceBits {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => status: ReadWrite<u8>),
        (0x0003 => error: ReadWrite<u8>),
        (0x0004 => protocol_specific: ReadWrite<u32>),
        (0x0008 => @END),
    }
}

impl FisSetDeviceBits {
    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        self.fis_type.write(t);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
    }

    pub fn i(&self) -> bool {
        self.flags.is_set(FisFlags::I)
    }

    pub fn set_i(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::I::SET
        } else {
            FisFlags::I::CLEAR
        });
    }

    pub fn n(&self) -> bool {
        self.flags.is_set(FisFlags::N)
    }

    pub fn set_n(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::N::SET
        } else {
            FisFlags::N::CLEAR
        });
    }

    pub fn status(&self) -> u8 {
        self.status.get()
    }

    pub fn set_status(&mut self, s: u8) {
        self.status.set(s);
    }

    pub fn error(&self) -> u8 {
        self.error.get()
    }

    pub fn set_error(&mut self, e: u8) {
        self.error.set(e);
    }
}

register_structs! {
    pub FisDmaSetup {
        (0x0000 => fis_type: ReadWrite<u8, FisType::Register>),
        (0x0001 => flags: ReadWrite<u8, FisFlags::Register>),
        (0x0002 => _rsv1),
        (0x0004 => dma_buf_id_low: ReadWrite<u32>),
        (0x0008 => dma_buf_id_high: ReadWrite<u32>),
        (0x000C => _rsv2),
        (0x0010 => dma_buf_offset: ReadWrite<u32>),
        (0x0014 => tx_count: ReadWrite<u32>),
        (0x0018 => @END),
    }
}

impl FisDmaSetup {
    pub fn fis_type(&self) -> FisType::TYPE::Value {
        self.fis_type.read_as_enum(FisType::TYPE).unwrap()
    }

    pub fn set_fis_type(&mut self, t: FieldValue<u8, FisType::Register>) {
        self.fis_type.write(t);
    }

    pub fn pm_port(&self) -> usize {
        self.flags.read(FisFlags::PM_PORT) as usize
    }

    pub fn set_pm_port(&mut self, port: usize) {
        self.flags.modify(FisFlags::PM_PORT.val(port as u8))
    }

    pub fn i(&self) -> bool {
        self.flags.is_set(FisFlags::I)
    }

    pub fn set_i(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::I::SET
        } else {
            FisFlags::I::CLEAR
        });
    }

    pub fn d(&self) -> bool {
        self.flags.is_set(FisFlags::D)
    }

    pub fn set_d(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::D::SET
        } else {
            FisFlags::D::CLEAR
        });
    }

    pub fn a(self) -> bool {
        self.flags.is_set(FisFlags::A)
    }

    pub fn set_a(&mut self, i: bool) {
        self.flags.modify(if i {
            FisFlags::A::SET
        } else {
            FisFlags::A::CLEAR
        });
    }

    pub fn buf_id(&self) -> u64 {
        *0u64
            .set_bits(0..32, self.dma_buf_id_low.get() as u64)
            .set_bits(32..64, self.dma_buf_id_high.get() as u64)
    }

    pub fn set_buf_id(&mut self, id: u64) {
        self.dma_buf_id_low.set(id.get_bits(0..32) as u32);
        self.dma_buf_id_high.set(id.get_bits(32..64) as u32);
    }

    pub fn buf_offset(&self) -> usize {
        self.dma_buf_offset.get() as usize
    }

    pub fn set_buf_offset(&mut self, off: u32) {
        self.dma_buf_offset.set(off);
    }

    pub fn tx_count(&self) -> usize {
        self.tx_count.get() as usize
    }

    pub fn set_tx_count(&mut self, c: u32) {
        self.tx_count.set(c);
    }
}

#[repr(C)]
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
