use crate::drivers::block::ahci::reg::FisRegH2D;
use crate::kernel::mm::PhysAddr;
use bit_field::BitField;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::ReadWrite;
use tock_registers::{register_bitfields, register_structs, LocalRegisterCopy};

register_bitfields! [
    u16,

    pub HbaCmdHeaderFlags [
        FIS_LENGTH OFFSET(0) NUMBITS(4),
        A OFFSET(5) NUMBITS(1),
        W OFFSET(6) NUMBITS(1),
        P OFFSET(7) NUMBITS(1),
        R OFFSET(8) NUMBITS(1),
        B OFFSET(9) NUMBITS(1),
        C OFFSET(10) NUMBITS(1),
        PORT_MULTIPLIER OFFSET(12) NUMBITS(4),
    ],
];

register_bitfields! [
    u32,

    pub HbaPrdtEntryFlags [
        DATA_BYTE_COUNT OFFSET(0) NUMBITS(22),
        INTERRUPT_ENABLED OFFSET(31) NUMBITS(1),
    ]
];

register_structs! {
    pub HbaCmdHeader {
        (0x000 => pub flags: ReadWrite<u16, HbaCmdHeaderFlags::Register>),
        (0x002 => pub prdtl: ReadWrite<u16>),
        (0x004 => pub prdbc: ReadWrite<u32>),
        (0x008 => pub ctb: ReadWrite<u64>),
        (0x010 => _reserved),
        (0x020 => @END),
    }
}

impl HbaCmdHeader {
    pub fn flags(&self) -> LocalRegisterCopy<u16, HbaCmdHeaderFlags::Register> {
        self.flags.extract()
    }

    pub fn set_flags(&mut self, f: LocalRegisterCopy<u16, HbaCmdHeaderFlags::Register>) {
        self.flags.set(f.get())
    }

    pub fn prdtl(&self) -> usize {
        self.prdtl.get() as usize
    }

    pub fn set_prdtl(&mut self, v: usize) {
        self.prdtl.set(v as u16)
    }

    pub fn prd_byte_count(&self) -> usize {
        self.prdbc.get() as usize
    }

    pub fn set_prd_byte_count(&mut self, v: usize) {
        self.prdbc.set(v as u32)
    }

    pub fn cmd_tbl_base_addr(&self) -> PhysAddr {
        PhysAddr(self.ctb.get() as usize)
    }

    pub fn set_cmd_tbl_base_addr(&mut self, a: PhysAddr) {
        self.ctb.set(a.0 as u64)
    }

    pub fn cmd_tbl(&self) -> &mut HbaCmdTbl {
        unsafe { self.cmd_tbl_base_addr().to_virt().read_mut::<HbaCmdTbl>() }
    }
}

register_structs! {
    pub HbaCmdTbl {
        (0x000 => cfis: [ReadWrite<u8>; 64]),
        (0x040 => acmd: [ReadWrite<u8>; 16]),
        (0x050 => _reserved),
        (0x080 => prdt_entry: [HbaPrdtEntry; 1]),
        (0x090 => @END),
    }
}

impl HbaCmdTbl {
    pub fn cfis_as_h2d_mut(&mut self) -> &mut FisRegH2D {
        unsafe { &mut *(self.cfis.as_mut_ptr() as *mut FisRegH2D) }
    }

    pub fn prdt_entry_mut(&mut self, i: usize) -> &mut HbaPrdtEntry {
        unsafe { &mut *self.prdt_entry.as_mut_ptr().offset(i as isize) }
    }

    pub fn reset(&mut self) {
        self.cfis.iter().for_each(|e| {
            e.set(0);
        });
        self.acmd.iter().for_each(|e| {
            e.set(0);
        });
        self._reserved.fill(0);
    }
}

register_structs! {
    pub HbaPrdtEntry {
        (0x000 => dba: ReadWrite<u32>),
        (0x004 => dbau: ReadWrite<u32>),
        (0x008 => _reserved),
        (0x00C => flags: ReadWrite<u32, HbaPrdtEntryFlags::Register>),
        (0x010 => @END),
    }
}

impl HbaPrdtEntry {
    pub fn database_address(&self) -> PhysAddr {
        PhysAddr(unsafe { self.dba.get() as usize | ((self.dbau.get() as usize) << 32) })
    }

    pub fn set_database_address(&mut self, addr: PhysAddr) {
        unsafe {
            self.dba.set(addr.0 as u32);
            self.dbau.set((addr.0 >> 32) as u32);
        }
    }

    pub fn data_byte_count(&self) -> usize {
        self.flags.read(HbaPrdtEntryFlags::DATA_BYTE_COUNT) as usize
    }

    pub fn set_data_byte_count(&mut self, b: usize) {
        self.flags
            .modify(HbaPrdtEntryFlags::DATA_BYTE_COUNT.val(b as u32));
    }

    pub fn interrupt_on_completion(&self) -> bool {
        self.flags.is_set(HbaPrdtEntryFlags::INTERRUPT_ENABLED)
    }

    pub fn set_interrupt_on_completion(&mut self, i: bool) {
        let v = if i {
            HbaPrdtEntryFlags::INTERRUPT_ENABLED::SET
        } else {
            HbaPrdtEntryFlags::INTERRUPT_ENABLED::CLEAR
        };
        self.flags.modify(v);
    }

    pub fn reset(&mut self) {
        self.set_database_address(PhysAddr(0));
        self._reserved.fill(0);
        self.flags.set(0);
    }
}
