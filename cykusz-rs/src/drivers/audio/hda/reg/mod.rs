#![allow(dead_code)]

#[macro_use]
pub mod macros;
pub(in crate::drivers::audio::hda) mod verb;

use bit_field::BitField;
use core::ops::{Deref, DerefMut};
use tock_registers::interfaces::{ReadWriteable, Readable};
use tock_registers::registers::{ReadOnly, ReadWrite};
use tock_registers::{
    register_bitfields, register_structs, LocalRegisterCopy, RegisterLongName, UIntLike,
};

register_bitfields! [
    u8,

    pub CorbCtl [
        CMEIE OFFSET(0) NUMBITS(1),
        CORBRUN OFFSET(1) NUMBITS(1),
    ],
    pub CorbStatus [
        CMEI OFFSET(0) NUMBITS(1),
    ],
    pub CorbSize [
        CORBSIZE OFFSET(0) NUMBITS(2) [
            Entries2 = 0,
            Entries16 = 1,
            Entries256 = 2,
        ],
        CORBSZCAP OFFSET(4) NUMBITS(4) [],
    ],
    pub RirbCtl [
        RINTCTL OFFSET(0) NUMBITS(1),
        RIRBDMAEN OFFSET(1) NUMBITS(1),
        RIRBOIC OFFSET(2) NUMBITS(1),
    ],
    pub RirbStatus [
        RINTFL OFFSET(0) NUMBITS(1),
        RIRBOIS OFFSET(2) NUMBITS(1),
    ],
    pub RirbSize [
        RIRBSIZE OFFSET(0) NUMBITS(2) [
            Entries2 = 0,
            Entries16 = 1,
            Entries256 = 2,
        ],
        RIRBSZCAP OFFSET(4) NUMBITS(4) [],
    ],
    pub StreamStatus [
        BCIS OFFSET(2) NUMBITS(1) [],
        FIFOE OFFSET(3) NUMBITS(1) [],
        DESE OFFSET(4) NUMBITS(1) [],
        FIFORDY OFFSET(5) NUMBITS(1) [],
    ],
    pub StreamControlLow [
        SRST OFFSET(0) NUMBITS(1) [],
        RUN OFFSET(1) NUMBITS(1) [],
        IOCE OFFSET(2) NUMBITS(1) [],
        FEIE OFFSET(3) NUMBITS(1) [],
        DEIE OFFSET(4) NUMBITS(1) [],
    ],
    pub StreamControlHigh [
        STRIPE OFFSET(0) NUMBITS(2) [
            SDO1 = 0,
            SDO2 = 1,
            SDO4 = 2,
        ],
        TP OFFSET(2) NUMBITS(1) [],
        DIR OFFSET(3) NUMBITS(1) [
            InputEngine = 0,
            OutputEngine = 1,
        ],
        STRM OFFSET(4) NUMBITS(4) [],
    ],
];

register_bitfields! [
    u16,

    pub GCap [
        BIT64_OK OFFSET(0) NUMBITS(1) [],
        NSDO OFFSET(1) NUMBITS(2) [ // Number of Serial Data Out Signals
            Sdo1 = 0,
            Sdo2 = 1,
            Sdo4 = 2,
            Reserved = 3,
        ],
        BSS OFFSET(3) NUMBITS(5) [], // Number of Bidirectional Streams
        ISS OFFSET(8) NUMBITS(4) [], // Number of Input Streams
        OSS OFFSET(12) NUMBITS(4) [], // Number of Output Streams
    ],
    pub Wakeen [
        SDIWEN OFFSET(0) NUMBITS(15),
    ],
    pub StateSt [
        SDIWAKE OFFSET(0) NUMBITS(15),
    ],
    pub GSts [
        FSTS OFFSET(1) NUMBITS(1),
    ],
    pub CorbWP [
        CORBWP OFFSET(0) NUMBITS(8),
    ],
    pub CorbRP [
        CORBRP OFFSET(0) NUMBITS(8),
        CORBRPRESET OFFSET(15) NUMBITS(1),
    ],
    pub RirbWP [
        RIRBWP OFFSET(0) NUMBITS(8),
        RIRBWPRST OFFSET(15) NUMBITS(1),
    ],
    pub RirbIntCnt [
        RINTCNT OFFSET(0) NUMBITS(8),
    ],
    pub ImmCmdStatus [
        ICB OFFSET(0) NUMBITS(1),
        IRV OFFSET(1) NUMBITS(1),
        IMMVER OFFSET(2) NUMBITS(1),
        IRRUNSOL OFFSET(3) NUMBITS(1),
        IRRADD OFFSET(4) NUMBITS(4),
    ],
    pub StreamLastValidIdx [
        LVI OFFSET(0) NUMBITS(8),
    ],
    pub StreamFormat [
        CHAN OFFSET(0) NUMBITS(4) [],
        BITS OFFSET(4) NUMBITS(3) [
            BITS8 = 0,
            BITS16 = 1,
            BITS20 = 2,
            BITS24 = 3,
            BITS32 = 4,
        ],
        DIV OFFSET(8) NUMBITS(3) [],
        MULT OFFSET(11) NUMBITS(3) [
            NONE = 0,
            X2 = 1,
            X3 = 2,
            X4 = 3
        ],
        BASE OFFSET(14) NUMBITS(1) [
            KHZ48 = 0,
            KHZ44 = 1,
        ]
    ]
];

register_bitfields! [
    u32,

    pub GCtl [
        CRST OFFSET(0) NUMBITS(1),   // Controller Reset
        FCNTRL OFFSET(1) NUMBITS(1), // Flush Control
        UNSOL OFFSET(8) NUMBITS(1),  // Accept Unsolicited Response Enable
    ],
    pub IntCtl [
        SIE OFFSET(0) NUMBITS(30), // Stream Interrupt Enable
        CIE OFFSET(30) NUMBITS(1), // Controller Interrupt Enable
        GIE OFFSET(31) NUMBITS(1), // Global Interrupt Enable
    ],
    pub IntSts [
        SIS OFFSET(0) NUMBITS(30), // Stream Interrupt Status
        CIS OFFSET(30) NUMBITS(1), // Controller Interrupt Status
        GIS OFFSET(31) NUMBITS(1), // Global Interrupt Status
    ],
    pub SSync [
        SSYNC OFFSET(0) NUMBITS(30),
    ],
    pub DplBase [
        DMAPOSENABLED OFFSET(0) NUMBITS(1),
        DPLBASE OFFSET(0) NUMBITS(32),
    ],
    pub BufferDescriptorListEntryIOC [
        IOC OFFSET(0) NUMBITS(1),
    ],
    pub StreamControl [
        SRST OFFSET(0) NUMBITS(1) [],
        RUN OFFSET(1) NUMBITS(1) [],
        IOCE OFFSET(2) NUMBITS(1) [],
        FEIE OFFSET(3) NUMBITS(1) [],
        DEIE OFFSET(4) NUMBITS(1) [],

        STRIPE OFFSET(16) NUMBITS(2) [
            SDO1 = 0,
            SDO2 = 1,
            SDO4 = 2,
        ],
        TP OFFSET(18) NUMBITS(1) [],
        DIR OFFSET(19) NUMBITS(1) [
            InputEngine = 0,
            OutputEngine = 1,
        ],
        STRM OFFSET(20) NUMBITS(4) [],
    ]
];

#[derive(Copy, Clone)]
pub struct WrapLocal<T: UIntLike, R: RegisterLongName = ()>(pub LocalRegisterCopy<T, R>);
pub struct WrapRW<T: UIntLike, R: RegisterLongName = ()>(pub ReadWrite<T, R>);
pub struct WrapRO<T: UIntLike, R: RegisterLongName = ()>(pub ReadOnly<T, R>);

impl<T: UIntLike, R: RegisterLongName> Deref for WrapRW<T, R> {
    type Target = ReadWrite<T, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: UIntLike, R: RegisterLongName> DerefMut for WrapRW<T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: UIntLike, R: RegisterLongName> WrapRO<T, R> {
    pub fn get_local(&self) -> WrapLocal<T, R> {
        WrapLocal(LocalRegisterCopy::new(self.0.get()))
    }
}

impl<T: UIntLike, R: RegisterLongName> WrapRW<T, R> {
    pub fn get_local(&self) -> WrapLocal<T, R> {
        WrapLocal(LocalRegisterCopy::new(self.0.get()))
    }
}

impl<T: UIntLike, R: RegisterLongName> Deref for WrapRO<T, R> {
    type Target = ReadOnly<T, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: UIntLike, R: RegisterLongName> Deref for WrapLocal<T, R> {
    type Target = LocalRegisterCopy<T, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: UIntLike, R: RegisterLongName> DerefMut for WrapLocal<T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: UIntLike, R: RegisterLongName> From<T> for WrapLocal<T, R> {
    fn from(value: T) -> Self {
        WrapLocal(LocalRegisterCopy::<T, R>::new(value))
    }
}

macro_rules! impl_uint_from_wrap_local {
    ($t:ty) => {
        impl<R: RegisterLongName> From<WrapLocal<$t, R>> for $t {
            fn from(value: WrapLocal<$t, R>) -> Self {
                value.0.into()
            }
        }
        impl<R: RegisterLongName> WrapLocal<$t, R> {
            pub fn new() -> WrapLocal<$t, R> {
                WrapLocal(LocalRegisterCopy::<$t, R>::new(0))
            }

            pub fn get_byte(&self, byte: usize) -> u8 {
                if size_of::<$t>() >= byte {
                    panic!("Byte out of range")
                }
                self.0.get().get_bits(byte * 8..byte * 8 + 8) as u8
            }
        }
    };
}

impl_uint_from_wrap_local!(u8);
impl_uint_from_wrap_local!(u16);
impl_uint_from_wrap_local!(u32);
impl_uint_from_wrap_local!(u64);
impl_uint_from_wrap_local!(usize);

register_structs! {
    pub Regs {
        (0x0000 => pub gcap: WrapRO<u16, GCap::Register>),
        (0x0002 => pub vmin: ReadOnly<u8>),
        (0x0003 => pub vmaj: ReadOnly<u8>),
        (0x0004 => pub outplay: ReadOnly<u16>),
        (0x0006 => pub inplay: ReadOnly<u16>),
        (0x0008 => pub gctl: WrapRW<u32, GCtl::Register>),
        (0x000C => pub wakeen: WrapRW<u16, Wakeen::Register>),
        (0x000E => pub statest: WrapRW<u16, StateSt::Register>),
        (0x0010 => pub gsts: WrapRW<u16, GSts::Register>),
        (0x0012 => _rsv1),
        (0x0018 => pub outstrmpay: ReadOnly<u16>),
        (0x001A => pub instrmpay: ReadOnly<u16>),
        (0x001C => _rsv2),
        (0x0020 => pub intctl: WrapRW<u32, IntCtl::Register>),
        (0x0024 => pub intsts: WrapRO<u32, IntSts::Register>),
        (0x0028 => _rsv3),
        (0x0030 => pub wallclk: ReadOnly<u32>),
        (0x0034 => _rsv4),
        (0x0038 => pub ssync: WrapRW<u32, SSync::Register>),
        (0x003C => _rsv5),
        (0x0070 => pub dpllowbase: WrapRW<u32, DplBase::Register>),
        (0x0074 => pub dplupbase: ReadWrite<u32>),
        (0x0078 => @END),
    }
}

register_structs! {
    pub Corb { // @0x0040
        (0x0000 => pub lowbase: ReadWrite<u32>),
        (0x0004 => pub upbase: ReadWrite<u32>),
        (0x0008 => pub wp: WrapRW<u16, CorbWP::Register>),
        (0x000A => pub rp: WrapRW<u16, CorbRP::Register>),
        (0x000C => pub ctl: WrapRW<u8, CorbCtl::Register>),
        (0x000D => pub status: WrapRW<u8, CorbStatus::Register>),
        (0x000E => pub size: WrapRW<u8, CorbSize::Register>),
        (0x000F => _rsv),
        (0x0010 => @END),
    }
}

register_structs! {
    pub Rirb { // @0x0050
        (0x0000 => pub lowbase: ReadWrite<u32>),
        (0x0004 => pub upbase: ReadWrite<u32>),
        (0x0008 => pub wp: WrapRW<u16, RirbWP::Register>),
        (0x000A => pub intcnt: WrapRW<u16, RirbIntCnt::Register>),
        (0x000C => pub ctl: WrapRW<u8, RirbCtl::Register>),
        (0x000D => pub status: WrapRW<u8, RirbStatus::Register>),
        (0x000E => pub size: WrapRW<u8, RirbSize::Register>),
        (0x000F => _rsv),
        (0x0010 => @END),
    }
}

register_structs! {
    pub Immediate { // @0x0060
        (0x0000 => pub output: ReadWrite<u32>),
        (0x0004 => pub input: ReadWrite<u32>),
        (0x0008 => pub status: WrapRW<u16, ImmCmdStatus::Register>),
        (0x000A => _rsv),
        (0x0010 => @END),
    }
}

register_structs! {
    pub Stream {
        (0x0000 => pub ctllow: WrapRW<u8, StreamControlLow::Register>),
        (0x0001 => _rsv),
        (0x0002 => pub ctlhigh: WrapRW<u8, StreamControlHigh::Register>),
        (0x0003 => pub status: WrapRW<u8, StreamStatus::Register>),
        (0x0004 => pub linkpos: ReadOnly<u32>),
        (0x0008 => pub buflen: ReadWrite<u32>),
        (0x000C => pub lastvali: WrapRW<u16, StreamLastValidIdx::Register>),
        (0x000E => _rsv1),
        (0x0010 => pub fifos: ReadOnly<u16>),
        (0x0012 => pub fmt: WrapRW<u16, StreamFormat::Register>),
        (0x0014 => _rsv2),
        (0x0018 => pub bdpllowbase: ReadWrite<u32>),
        (0x001C => pub bdplupbase: ReadWrite<u32>),
        (0x0020 => @END),
    }
}

register_structs! {
    pub BufferDescriptorListEntry {
        (0x0000 => pub address: ReadWrite<u64>),
        (0x0008 => pub length: ReadWrite<u32>),
        (0x000C => pub ioc: WrapRW<u32, BufferDescriptorListEntryIOC::Register>),
        (0x0010 => @END),
    }
}

impl_wrap! {
    [WrapRO, WrapLocal],

    GCap as u16,

    bool get [
        is_bit64_ok(BIT64_OK);
    ],

    enum get [
        nsdo(NSDO);
    ],

    int get [
        bss(BSS);
        iss(ISS);
        oss(OSS);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    GCtl as u32,

    bool get_set [
        is_crst(CRST);
        is_fcntrl(FCNTRL);
        is_unsol(UNSOL);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    Wakeen as u16,

    int get_set [
        sdiwen(SDIWEN);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StateSt as u16,

    int get_set [
        sdiwake(SDIWAKE);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    GSts as u16,

    int get_set [
        fsts(FSTS);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StreamStatus as u8,

    bool get_set [
        is_bcis(BCIS);
        is_fifoe(FIFOE);
        is_dese(DESE);
        is_fifordy(FIFORDY);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StreamControlLow as u8,

    bool get_set [
        is_srst(SRST);
        is_run(RUN);
        is_ioce(IOCE);
        is_feie(FEIE);
        is_deie(DEIE);
    ],
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StreamControlHigh as u8,

    bool get_set [
        is_tp(TP);
    ],
    enum get [
        stripe(STRIPE);
        dir(DIR);
    ],

    int get_set [
        stream(STRM);
    ]
}

impl_wrap! {
    [WrapLocal],

    StreamControl as u32,

    bool get_set [
        is_srst(SRST);
        is_run(RUN);
        is_ioce(IOCE);
        is_feie(FEIE);
        is_deie(DEIE);
        is_tp(TP);
    ],
    enum get [
        stripe(STRIPE);
        dir(DIR);
    ],
    int get_set [
        stream(STRM);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    IntCtl as u32,

    int get_set [
        sie(SIE);
    ],

    bool get_set [
        is_cie(CIE);
        is_gie(GIE);
    ]
}

impl_wrap! {
    [WrapRO, WrapLocal],

    IntSts as u32,

    int get [
        sis(SIS);
    ],

    bool get [
        is_cis(CIS);
        is_gis(GIS);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    SSync as u32,

    int get_set [
        ssync(SSYNC);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    CorbWP as u16,

    int get_set [
        wp(CORBWP);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    CorbRP as u16,

    int get_set [
        rp(CORBRP);
    ],

    bool get_set [
        is_rp_reset(CORBRPRESET);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    CorbCtl as u8,

    bool get_set [
        is_cmeie(CMEIE);
        is_run(CORBRUN);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    CorbStatus as u8,

    bool get_set [
        is_cmei(CMEI);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    CorbSize as u8,

    enum get_set [
        size(CORBSIZE);
    ],
    int get_set [
        size_cap(CORBSZCAP);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    RirbWP as u16,

    int get_set [
        wp(RIRBWP);
    ],

    bool get_set [
        is_wp_reset(RIRBWPRST);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    RirbIntCnt as u16,

    int get_set [
        int_cnt(RINTCNT);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    RirbCtl as u8,

    bool get_set [
        is_int_ctl(RINTCTL);
        is_dma_en(RIRBDMAEN);
        is_oic(RIRBOIC);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    RirbStatus as u8,

    bool get_set [
        int_fl(RINTFL);
        ois(RIRBOIS);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    RirbSize as u8,

    enum get_set [
        size(RIRBSIZE);
    ],

    int get_set [
        size_cap(RIRBSZCAP);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    ImmCmdStatus as u16,

    bool get_set [
        is_icb(ICB);
        is_irv(IRV);
        is_immver(IMMVER);
        is_irrunsol(IRRUNSOL);
    ],

    int get_set [
        irradd(IRRADD);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    DplBase as u32,

    bool get_set [
        is_dma_pos_enabled(DMAPOSENABLED);
    ],

    int get_set [
        base(DPLBASE);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StreamLastValidIdx as u16,

    int get_set [
        last_valid_idx(LVI);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    StreamFormat as u16,

    int get_set [
        channel(CHAN);
        div(DIV);
    ],

    enum get_set [
        bits(BITS);
        mult(MULT);
        base(BASE);
    ]
}

impl_wrap! {
    [WrapRW, WrapLocal],

    BufferDescriptorListEntryIOC as u32,

    bool get_set [
        is_ioc(IOC);
    ],
}
