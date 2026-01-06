use alloc::boxed::Box;
use bit_field::BitField;
use tock_registers::{register_bitfields, RegisterLongName};
// This is way too overengineered but who cares

register_bitfields! [
    u64,

    pub GetParameterVendorIDReg [
        DEVICE_ID OFFSET(0) NUMBITS(16),
        VENDOR_ID OFFSET(16) NUMBITS(16),
    ],
    pub GetParameterRevisionIDReg [
        STEPPING_ID OFFSET(0) NUMBITS(8),
        REVISION_ID OFFSET(8) NUMBITS(8),
        MIN_REV OFFSET(16) NUMBITS(4),
        MAJ_REV OFFSET(20) NUMBITS(4),
    ],
    pub GetParameterNodeCountReg [
        TOTAL_COUNT OFFSET(0) NUMBITS(8),
        STARTING_NODE OFFSET(16) NUMBITS(8),
    ],
    pub GetParameterFunctionGroupTypeReg [
        NODE_TYPE OFFSET(0) NUMBITS(8),
        UNSOL_CAPABLE OFFSET(8) NUMBITS(1),
    ],
    pub GetParameterAudioFunctionGroupCapReg [
        OUTPUT_DELAY OFFSET(0) NUMBITS(4),
        INPUT_DELAY OFFSET(8) NUMBITS(4),
        BEEP_GEN OFFSET(16) NUMBITS(1),
    ],
    pub GetParameterAudioWidgetCapReg [
        CHAN_COUNT_LSB OFFSET(0) NUMBITS(1) [],
        IN_AMP_PRESENT OFFSET(1) NUMBITS(1) [],
        OUT_AMP_PRESENT OFFSET(2) NUMBITS(1) [],
        AMP_PARAM_OVERRIDE OFFSET(3) NUMBITS(1) [],
        FORMAT_OVERRIDE OFFSET(4) NUMBITS(1) [],
        STRIPE OFFSET(5) NUMBITS(1) [],
        PROC_WIDGET OFFSET(6) NUMBITS(1) [],
        UNSOL_CAPABLE OFFSET(7) NUMBITS(1) [],
        CONN_LIST OFFSET(8) NUMBITS(1) [],
        DIGITAL OFFSET(9) NUMBITS(1) [],
        POWER_CNTRL OFFSET(10) NUMBITS(1) [],
        LR_SWAP OFFSET(11) NUMBITS(1) [],
        CP_CAPS OFFSET(12) NUMBITS(1) [],
        CHAN_COUNT_EXT OFFSET(13) NUMBITS(3) [],
        DELAY OFFSET(16) NUMBITS(4) [],
        TYPE OFFSET(20) NUMBITS(4) [
            AudioOutput = 0x00,
            AudioInput = 0x01,
            AudioMixer = 0x02,
            AudioSelector = 0x03,
            PinComplex = 0x04,
            PowerWidget = 0x05,
            VolumeKnobWidget = 0x06,
            BeepGeneratorWidget = 0x07,
            VendorDefinedAudioWidget = 0x0f,
        ],
    ],
    pub GetParameterSupportedPCMRatesReg [
        RATES OFFSET(0) NUMBITS(12),
        B8 OFFSET(16) NUMBITS(1),
        B16 OFFSET(17) NUMBITS(1),
        B20 OFFSET(18) NUMBITS(1),
        B24 OFFSET(19) NUMBITS(1),
        B32 OFFSET(20) NUMBITS(1),
    ],
    pub GetParameterSupportedStreamFormatsReg [
        PCM OFFSET(0),
        FLOAT32 OFFSET(1),
        AC3 OFFSET(2),
    ],
    pub GetParameterPinCapReg [
        IMPEDANCE_SENSE_CAPABLE OFFSET(0) NUMBITS(1),
        TRIGGER_REQD OFFSET(1) NUMBITS(1),
        PRESENCE_DETECT_CAPABLE OFFSET(2) NUMBITS(1),
        HEADPHONE_DRIVE_CAPABLE OFFSET(3) NUMBITS(1),
        OUTPUT_CAPABLE OFFSET(4) NUMBITS(1),
        INPUT_CAPABLE OFFSET(5) NUMBITS(1),
        BALANCED_IO_PINS OFFSET(6) NUMBITS(1),
        HDMI OFFSET(7) NUMBITS(1),
        VREF_CONTROL OFFSET(8) NUMBITS(8),
        EAPD_CAPABLE OFFSET(16) NUMBITS(1),
        DP OFFSET(24) NUMBITS(1),
        HBR OFFSET(27) NUMBITS(1),
    ],
    pub GetParameterAmplifierCapReg [
        OFFSET OFFSET(0) NUMBITS(7),
        NUM_STEPS OFFSET(8) NUMBITS(7),
        STEP_SIZE OFFSET(16) NUMBITS(7),
        MUTE_CAPABLE OFFSET(31) NUMBITS(1),
    ],
    pub GetParameterConnectionListLengthReg [
        LENGTH OFFSET(0) NUMBITS(7),
        LONG_FORM OFFSET(7) NUMBITS(1),
    ],
    pub GetParameterSupportedPowerStatesReg [
        D0SUP OFFSET(0),
        D1SUP OFFSET(1),
        D2SUP OFFSET(2),
        D3SUP OFFSET(3),
        D3COLDSUP OFFSET(4),
        S3D3COLDSUP OFFSET(29),
        CLKSTOP OFFSET(30),
        EPSS OFFSET(31),
    ],
    pub GetParameterProcessingCapReg [
        BENING OFFSET(0) NUMBITS(1),
        NUM_COEFF OFFSET(8) NUMBITS(8),
    ],
    pub GetParameterGPIOCountReg [
        NUM_GPIOS OFFSET(0) NUMBITS(8),
        NUM_GPOS OFFSET(8) NUMBITS(8),
        NUM_GPIS OFFSET(16) NUMBITS(8),
        GPI_UNSOL OFFSET(30) NUMBITS(1),
        GPI_WAKE OFFSET(31) NUMBITS(1),
    ],
    pub GetParameterVolumeKnobCapReg [
        NUM_STEPS OFFSET(0) NUMBITS(7),
        DELTA OFFSET(7) NUMBITS(1),
    ],
    pub ConnectionListEntryReg [
        SHORT_ENTRY_0 OFFSET(0) NUMBITS(8),
        SHORT_ENTRY_1 OFFSET(8) NUMBITS(8),
        SHORT_ENTRY_2 OFFSET(16) NUMBITS(8),
        SHORT_ENTRY_3 OFFSET(24) NUMBITS(8),
        LONG_ENTRY_0 OFFSET(0) NUMBITS(16),
        LONG_ENTRY_1 OFFSET(16) NUMBITS(16),
    ],
    pub ProcessingStateReg [
        VALUE OFFSET(0) NUMBITS(8),
    ],
    pub GetAmplifierGainMutePayloadReg [
        INDEX OFFSET(0) NUMBITS(4),
        LEFT OFFSET(13) NUMBITS(1),
        OUTPUT OFFSET(15) NUMBITS(1),
    ],
    pub GetAmplifierGainMuteResponseReg [
        GAIN OFFSET(0) NUMBITS(7),
        MUTE OFFSET(7) NUMBITS(1),
    ],
    pub SetAmplifierGainMutePayloadReg [
        GAIN OFFSET(0) NUMBITS(7),
        MUTE OFFSET(7) NUMBITS(1),
        INDEX OFFSET(8) NUMBITS(4),
        SET_RIGHT_AMP OFFSET(12) NUMBITS(1),
        SET_LEFT_AMP OFFSET(13) NUMBITS(1),
        SET_INPUT_AMP OFFSET(14) NUMBITS(1),
        SET_OUTPUT_AMP OFFSET(15) NUMBITS(1),
    ],
    pub StreamFormatReg [
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
            x3 = 2,
            x4 = 3
        ],
        BASE OFFSET(14) NUMBITS(1) [
            KHZ48 = 0,
            KHZ44 = 1,
        ]
    ],
    pub SPDIFControlReg [
        DIG_EN OFFSET(0) NUMBITS(1),
        V OFFSET(1) NUMBITS(1),
        VCFG OFFSET(2) NUMBITS(1),
        PRE OFFSET(3) NUMBITS(1),
        CP OFFSET(4) NUMBITS(1),
        AUDIO OFFSET(5) NUMBITS(1),
        PRO OFFSET(6) NUMBITS(1),
        L OFFSET(7) NUMBITS(1),
        CC OFFSET(8) NUMBITS(7),
        ICT OFFSET(16) NUMBITS(4),
        KAE OFFSET(23) NUMBITS(1),
    ],
    pub PowerStateReg [
        PS_SET OFFSET(0) NUMBITS(4) [
            D0 = 0b0000,
            D1 = 0b0001,
            D2 = 0b0010,
            D3 = 0b0011,
            D3cold = 0b0100,

        ],
        PS_ACT OFFSET(4) NUMBITS(4) [],
        PS_ERROR OFFSET(8) NUMBITS(1) [],
        PS_CLK_STOP_OK OFFSET(9) NUMBITS(1) [],
    ],
    pub ChannelStreamIDReg [
        CHANNEL OFFSET(0) NUMBITS(4),
        STREAM OFFSET(4) NUMBITS(4),
    ],
    pub PinWidgetControlReg [
        VREF_EN OFFSET(0) NUMBITS(2),
        VREF_EN_2 OFFSET(2) NUMBITS(1),
        IN_ENABLE OFFSET(5) NUMBITS(1),
        OUT_ENABLE OFFSET(6) NUMBITS(1),
        HPHN_ENABLE OFFSET(7) NUMBITS(1),
    ],
    pub EnableUnsolReg [
        TAG OFFSET(0) NUMBITS(6),
        ENABLE OFFSET(7) NUMBITS(1),
    ],
    pub PinSenseReg [
        PRESENCE_DETECTED OFFSET(31),
    ],
    pub EAPDBTLEnableReg [
        BTL 0,
        EAPD 1,
        LR_SWAP 2,
    ],
    pub VolumeKnobReg [
        VOLUME OFFSET(0) NUMBITS(7),
        DIRECT OFFSET(7) NUMBITS(1),
    ],
    pub ImplementationIDReg [
        ASSEMBLY_ID OFFSET(0) NUMBITS(8),
        BOARD_SKU OFFSET(8) NUMBITS(8),
        BOARD_MANUFACTURER_ID OFFSET(16) NUMBITS(16),
        BOARD_IMPL_ID OFFSET(8) NUMBITS(24),
    ],
    pub ConfigurationDefaultReg [
        SEQUENCE OFFSET(0) NUMBITS(4) [],
        DEFAULT_ASSOCIATION OFFSET(4) NUMBITS(4) [],
        MISC OFFSET(8) NUMBITS(4) [
            JackDetectOverride = 0,
        ],
        COLOR OFFSET(12) NUMBITS(4) [
            Unknown = 0,
            Black = 0x1,
            Grey = 0x2,
            Blue = 0x3,
            Green = 0x4,
            Red = 0x5,
            Orange = 0x6,
            Yellow = 0x7,
            Purple = 0x8,
            Pink = 0x9,
            White = 0xE,
            Other = 0xF,
        ],
        CONNECTION_TYPE OFFSET(16) NUMBITS(4) [
            Unknown = 0x0,
            StereoMono1_8 = 0x1,
            StereoMono1_4 = 0x2,
            ATAPIInternal = 0x3,
            RCA = 0x4,
            Optical = 0x5,
            OtherDigital = 0x6,
            OtherAnalog = 0x7,
            MultichannelAnalog = 0x8,
            XLRProfessional = 0x9,
            RJ11 = 0xA,
            Combination = 0xB,
            Other = 0xF,
        ],
        DEFAULT_DEVICE OFFSET(20) NUMBITS(4) [
            LineOut = 0x0,
            Speaker = 0x1,
            HPOut = 0x2,
            CD = 0x3,
            SPDIFOut = 0x4,
            DigitalOtherOut = 0x5,
            ModemLineSide = 0x6,
            ModemHandsetSide = 0x7,
            LineIn = 0x8,
            AUX = 0x9,
            MicIn = 0xA,
            Telephony = 0xB,
            SPDIFIn = 0xC,
            DigitalOtherIn = 0xD,
            Other = 0xF,
        ],
        LOCATION1 OFFSET(24) NUMBITS(4) [
            NA = 0x0,
            Read = 0x1,
            Front = 0x2,
            Left = 0x3,
            Right = 0x4,
            Top = 0x5,
            Bottom = 0x6,
            Special1 = 0x7,
            Special2 = 0x8,
            Special3 = 0x9,
        ],
        LOCATION2 OFFSET(28) NUMBITS(2) [
            ExternalOnPrimaryChassis = 0x0,
            Internal = 0x1,
            SeparateChasis = 0x2,
            Other = 0x3,
        ],
        PORT_CONNECTIVITY OFFSET(30) NUMBITS(2) [
            ToJack = 0x0,
            NoPhysConnection = 0x1,
            FixedFunctionDevice = 0x2,
            ToJackAndInternalDev = 0x3,
        ]
    ],
    pub StripeControlReg [
        STRIPE_CONTROL OFFSET(0) NUMBITS(2),
        STRIPE_CAPABILITY OFFSET(20) NUMBITS(2),
    ],
    pub DataIslandPacketSizePayloadReg [
        PACKET_INDEX OFFSET(0) NUMBITS(3),
        ELD_BUFFER_SIZE OFFSET(3) NUMBITS(1),
    ],
    pub DataIslandPacketIndexReg [
        BYTE_INDEX OFFSET(0) NUMBITS(5),
        PACKET_INDEX OFFSET(5) NUMBITS(3),
    ],
    pub DataIslandPacketTXCtrl [
        XMIT_CTRL OFFSET(6) NUMBITS(2) [
            DisableTransmission = 0x0,
            TransmitOnce = 0x2,
            TransmitBestEffort = 0x3,
        ]
    ],
    pub ContentProtectionReg [
        CP OFFSET(0) NUMBITS(2) [
            DontCare = 0x0,
            OFF = 0x2,
            ON = 0x3,
        ],
        UR_TAG OFFSET(3) NUMBITS(5) [],
        READY OFFSET(8) NUMBITS(1) [],
        CES OFFSET(9) NUMBITS(1) [],
    ],
    pub AudioSamplePacketChannelMappingReg [
        ASP_SLOT OFFSET(0) NUMBITS(4),
        CONVERTER_CHANNEL OFFSET(4) NUMBITS(4),
    ]
];

use crate::drivers::audio::hda::reg::WrapLocal;
use GetParameterAmplifierCapReg as GetParameterInputAmplifierCapReg;
use GetParameterAmplifierCapReg as GetParameterOutputAmplifierCapReg;

impl<R: RegisterLongName> From<WrapLocal<u64, R>> for u32 {
    fn from(value: WrapLocal<u64, R>) -> Self {
        value.get() as u32
    }
}

impl_wrap!(
    [WrapLocal],

    GetParameterVendorIDReg as u64,

    int get [
        vendor_id(VENDOR_ID);
        device_id(DEVICE_ID);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterRevisionIDReg as u64,

    int get [
        stepping_id(STEPPING_ID);
        revision_id(STEPPING_ID);
        min_rev(MIN_REV);
        maj_rev(MAJ_REV);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterNodeCountReg as u64,

    int get [
        total_count(TOTAL_COUNT);
        starting_node(STARTING_NODE);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterAudioFunctionGroupCapReg as u64,

    int get [
        input_delay(INPUT_DELAY);
        output_delay(OUTPUT_DELAY);
    ],

    bool get [
        is_beep_gen(BEEP_GEN);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterAudioWidgetCapReg as u64,

    bool get [
        is_chan_count_lsb(CHAN_COUNT_LSB);
        is_in_amp_present(IN_AMP_PRESENT);
        is_out_amp_present(OUT_AMP_PRESENT);
        is_amp_param_override(AMP_PARAM_OVERRIDE);
        is_format_override(FORMAT_OVERRIDE);
        is_stripe(STRIPE);
        is_proc_widget(PROC_WIDGET);
        is_unsol_capable(UNSOL_CAPABLE);
        is_conn_list(CONN_LIST);
        is_digital(DIGITAL);
        is_power_cntrl(POWER_CNTRL);
        is_lr_swap(LR_SWAP);
        is_cp_caps(CP_CAPS);
    ],

    int get [
        chan_count_ext(CHAN_COUNT_EXT);
        delay(DELAY);
    ],

    enum get [
        typ(TYPE);
    ],
);

impl_wrap!(
    [WrapLocal],

    GetParameterSupportedPCMRatesReg as u64,

    int get [
        rates(RATES);
    ],

    bool get [
        is_b8(B8);
        is_b16(B16);
        is_b20(B20);
        is_b24(B24);
        is_b32(B32);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterSupportedStreamFormatsReg as u64,

    bool get [
        is_pcm(PCM);
        is_float(FLOAT32);
        is_ac3(AC3);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterPinCapReg as u64,

    bool get [
        is_impedance_sense_capable(IMPEDANCE_SENSE_CAPABLE);
        is_trigger_reqd(TRIGGER_REQD);
        is_presence_detect_capable(PRESENCE_DETECT_CAPABLE);
        is_headphone_drive_capable(HEADPHONE_DRIVE_CAPABLE);
        is_output_capable(OUTPUT_CAPABLE);
        is_input_capable(INPUT_CAPABLE);
        is_balanced_io_pins(BALANCED_IO_PINS);
        is_hdmi(HDMI);
        is_eapd_capable(EAPD_CAPABLE);
        is_dp(DP);
        is_hbr(HBR);
    ],

    int get [
        vref_control(VREF_CONTROL);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterAmplifierCapReg as u64,

    int get [
        offset(OFFSET);
        num_steps(NUM_STEPS);
        step_size(STEP_SIZE);
    ],

    bool get [
        is_mute_capable(MUTE_CAPABLE);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterConnectionListLengthReg as u64,

    int get [
        length(LENGTH);
    ],

    bool get [
        is_long_form(LONG_FORM);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterSupportedPowerStatesReg as u64,

    bool get [
        is_d0sup(D0SUP);
        is_d1sup(D1SUP);
        is_d2sup(D2SUP);
        is_d3sup(D3SUP);
        is_d3coldsup(D3COLDSUP);
        is_s3d3coldsup(S3D3COLDSUP);
        is_clkstop(CLKSTOP);
        is_epss(EPSS);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterProcessingCapReg as u64,

    bool get [
        is_bening(BENING);
    ],

    int get [
        num_coeff(NUM_COEFF);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterGPIOCountReg as u64,

    bool get [
        is_gpi_unsol(GPI_UNSOL);
        is_gpi_wake(GPI_WAKE);
    ],
    int get [
        num_gpios(NUM_GPIOS);
        num_gpos(NUM_GPOS);
        num_gpis(NUM_GPIS);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetParameterVolumeKnobCapReg as u64,

    int get [
        num_steps(NUM_STEPS);
    ],

    bool get [
        is_delta(DELTA);
    ]
);

#[derive(Copy, Clone, Debug)]
pub enum FunctionGroupType {
    Audio,
    VendorDefinedModem,
    VendorDefined(u32),
    Invalid,
}

impl WrapLocal<u64, GetParameterFunctionGroupTypeReg::Register> {
    pub fn function_group_type(&self) -> FunctionGroupType {
        match self.read(GetParameterFunctionGroupTypeReg::NODE_TYPE) as u32 {
            0x0 => FunctionGroupType::Invalid,
            0x1 => FunctionGroupType::Audio,
            0x2 => FunctionGroupType::VendorDefinedModem,
            v @ 0x80..=0xFF => FunctionGroupType::VendorDefined(v),
            _ => panic!("Invalid function group type"),
        }
    }

    impl_methods!(
        mut GetParameterFunctionGroupTypeReg as u64,

        bool get [
            is_unsol_capable(UNSOL_CAPABLE);
        ]
    );
}

pub trait ConnectionListEntryInt {
    const IS_LONG: bool;
    const COUNT_PER_ENTRY: usize;
    fn from_u64(value: u64) -> Self;
    fn int(&self) -> u16;
}

impl ConnectionListEntryInt for u8 {
    const IS_LONG: bool = false;
    const COUNT_PER_ENTRY: usize = 4;
    fn from_u64(value: u64) -> Self {
        value as Self
    }

    fn int(&self) -> u16 {
        *self as u16
    }
}
impl ConnectionListEntryInt for u16 {
    const IS_LONG: bool = true;
    const COUNT_PER_ENTRY: usize = 2;
    fn from_u64(value: u64) -> Self {
        value as Self
    }

    fn int(&self) -> u16 {
        *self
    }
}
pub struct ConnectionListEntry<T: ConnectionListEntryInt>(T);

impl<T: ConnectionListEntryInt> From<u64> for ConnectionListEntry<T> {
    fn from(value: u64) -> Self {
        ConnectionListEntry(T::from_u64(value))
    }
}

pub trait ConnectionListEntryKind {
    fn is_range_of_nids(&self) -> bool;
    fn nids(&self) -> u16;
    fn is_valid(&self) -> bool;
}

impl<T: ConnectionListEntryInt> ConnectionListEntryKind for ConnectionListEntry<T> {
    fn is_range_of_nids(&self) -> bool {
        self.0.int().get_bit(size_of::<T>() * 8 - 1)
    }
    fn nids(&self) -> u16 {
        self.0.int().get_bits(0..(size_of::<T>() * 8 - 1))
    }
    fn is_valid(&self) -> bool {
        self.0.int() != 0
    }
}

impl WrapLocal<u64, ConnectionListEntryReg::Register> {
    impl_methods!(
        mut ConnectionListEntryReg as u64,

        type get [
            short_entry_0(SHORT_ENTRY_0) => ConnectionListEntry<u8>;
            short_entry_1(SHORT_ENTRY_1) => ConnectionListEntry<u8>;
            short_entry_2(SHORT_ENTRY_2) => ConnectionListEntry<u8>;
            short_entry_3(SHORT_ENTRY_3) => ConnectionListEntry<u8>;
            long_entry_0(LONG_ENTRY_0) => ConnectionListEntry<u16>;
            long_entry_1(LONG_ENTRY_1) => ConnectionListEntry<u16>;
        ]
    );

    pub fn short_entry(&self, idx: usize) -> ConnectionListEntry<u8> {
        match idx {
            0 => self.short_entry_0(),
            1 => self.short_entry_1(),
            2 => self.short_entry_2(),
            3 => self.short_entry_3(),
            _ => panic!("Invalid short entry index"),
        }
    }

    pub fn long_entry(&self, idx: usize) -> ConnectionListEntry<u16> {
        match idx {
            0 => self.long_entry_0(),
            1 => self.long_entry_1(),
            _ => panic!("Invalid long entry index"),
        }
    }

    pub fn entry<T: ConnectionListEntryInt>(&self, idx: usize) -> Box<dyn ConnectionListEntryKind> {
        if T::IS_LONG {
            Box::new(self.long_entry(idx))
        } else {
            Box::new(self.short_entry(idx))
        }
    }
}

pub enum ProcessingState {
    Off,
    On,
    Benign,
    Vendor(u32),
}

impl From<ProcessingState> for u64 {
    fn from(value: ProcessingState) -> Self {
        match value {
            ProcessingState::Off => 0,
            ProcessingState::On => 1,
            ProcessingState::Benign => 2,
            ProcessingState::Vendor(v) => v as u64,
        }
    }
}

impl WrapLocal<u64, ProcessingStateReg::Register> {
    pub fn processing_state(&self) -> ProcessingState {
        match self.read(ProcessingStateReg::VALUE) {
            0 => ProcessingState::Off,
            1 => ProcessingState::On,
            2 => ProcessingState::Benign,
            v @ 0x80..=0xFF => ProcessingState::Vendor(v as u32),
            _ => panic!("Invalid Processing State"),
        }
    }

    impl_set_method!(
        set_processing_state,
        ProcessingStateReg::VALUE => into ProcessingState
    );
}

impl_wrap!(
    [WrapLocal],

    GetAmplifierGainMutePayloadReg as u64,

    bool get_set [
        is_left(LEFT);
        is_output(OUTPUT);
    ],

    int get_set [
        index(INDEX);
    ]
);

impl_wrap!(
    [WrapLocal],

    GetAmplifierGainMuteResponseReg as u64,

    int get [
        gain(GAIN);
    ],

    bool get [
        is_mute(MUTE);
    ]
);

impl_wrap!(
    [WrapLocal],

    SetAmplifierGainMutePayloadReg as u64,

    int get_set [
        gain(GAIN);
        index(INDEX);
    ],

    bool get_set [
        is_mute(MUTE);
        is_set_right_amp(SET_RIGHT_AMP);
        is_set_left_amp(SET_LEFT_AMP);
        is_set_input_amp(SET_INPUT_AMP);
        is_set_output_amp(SET_OUTPUT_AMP);
    ]
);

impl_wrap!(
    [WrapLocal],

    StreamFormatReg as u64,

    enum get_set [
        bits(BITS);
        mult(MULT);
        base(BASE);
    ],

    int get_set [
        chan(CHAN);
        div(DIV);
    ]
);

impl_wrap! (
    [WrapLocal],

    SPDIFControlReg as u64,

    bool get_set [
        is_dig_en(DIG_EN);
        is_v(V);
        is_vcfg(VCFG);
        is_pre(PRE);
        is_cp(CP);
        is_audio(AUDIO);
        is_pro(PRO);
        is_l(L);
        is_cc(CC);
        is_ict(ICT);
        is_kae(KAE);
    ]
);

impl_wrap!(
    [WrapLocal],

    PowerStateReg as u64,

    enum get_set [
        ps_set(PS_SET);
    ],

    int get_set [
        ps_act(PS_ACT);
    ],

    bool get_set [
        is_ps_error(PS_ERROR);
        is_ps_clk_stop_ok(PS_CLK_STOP_OK);
    ]
);

impl_wrap!(
    [WrapLocal],

    ChannelStreamIDReg as u64,

    int get_set [
        channel(CHANNEL);
        stream(STREAM);
    ]
);

impl_wrap!(
    [WrapLocal],

    PinWidgetControlReg as u64,

    int get_set [
        vref_en(VREF_EN);
    ],

    bool get_set [
        is_vref_en_2(VREF_EN_2);
        is_in_enabled(IN_ENABLE);
        is_out_enabled(OUT_ENABLE);
        is_hphn_enabled(HPHN_ENABLE);
    ]
);

impl_wrap!(
    [WrapLocal],

    EnableUnsolReg as u64,

    int get_set [
        tag(TAG);
    ],

    bool get_set [
        is_enabled(ENABLE);
    ]
);

impl_wrap!(
    [WrapLocal],

    PinSenseReg as u64,

    bool get_set [
        is_presence_detected(PRESENCE_DETECTED);
    ]
);

impl_wrap!(
    [WrapLocal],

    EAPDBTLEnableReg as u64,

    bool get_set [
        is_btl(BTL);
        is_eapd(EAPD);
        is_lr_swap(LR_SWAP);
    ]
);

impl_wrap!(
    [WrapLocal],

    VolumeKnobReg as u64,

    int get_set [
        volume(VOLUME);
    ],

    bool get_set [
        is_direct(DIRECT);
    ]
);

impl_wrap!(
    [WrapLocal],

    ImplementationIDReg as u64,

    int get_set [
        assembly_id(ASSEMBLY_ID);
        board_sku(BOARD_SKU);
        board_manufacturer_id(BOARD_MANUFACTURER_ID);
        board_impl_id(BOARD_IMPL_ID);
    ]
);

impl WrapLocal<u64, ConfigurationDefaultReg::Register> {
    impl_methods!(
        mut ConfigurationDefaultReg as u64,

        int get_set [
            sequence(SEQUENCE);
            default_association(DEFAULT_ASSOCIATION);
        ],

        enum get_set [
            misc(MISC);
            color(COLOR);
            connection_type(CONNECTION_TYPE);
            default_device(DEFAULT_DEVICE);
            location1(LOCATION1);
            location2(LOCATION2);
            port_connectivity(PORT_CONNECTIVITY);
        ]
    );

    pub fn is_output(&self) -> bool {
        match self.default_device() {
            Some(
                ConfigurationDefaultReg::DEFAULT_DEVICE::Value::LineOut
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::Speaker
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::HPOut
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::CD
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::SPDIFOut
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::DigitalOtherOut
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::ModemLineSide,
            ) => true,
            _ => false,
        }
    }

    pub fn is_input(&self) -> bool {
        match self.default_device() {
            Some(
                ConfigurationDefaultReg::DEFAULT_DEVICE::Value::ModemHandsetSide
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::LineIn
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::AUX
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::MicIn
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::Telephony
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::SPDIFIn
                | ConfigurationDefaultReg::DEFAULT_DEVICE::Value::DigitalOtherIn,
            ) => true,
            _ => false,
        }
    }
}

impl_wrap!(
    [WrapLocal],

    StripeControlReg as u64,

    int get_set [
        stripe_control(STRIPE_CONTROL);
        stripe_capability(STRIPE_CAPABILITY);
    ]
);

impl_wrap!(
    [WrapLocal],

    DataIslandPacketSizePayloadReg as u64,

    int get_set [
        packet_index(PACKET_INDEX);
    ],

    bool get_set [
        is_eld_buffer_size(ELD_BUFFER_SIZE);
    ]
);

impl_wrap!(
    [WrapLocal],

    DataIslandPacketIndexReg as u64,

    int get_set [
        byte_index(BYTE_INDEX);
        packet_index(PACKET_INDEX);
    ]
);

impl_wrap!(
    [WrapLocal],

    DataIslandPacketTXCtrl as u64,

    enum get_set [
        xmit_ctrl(XMIT_CTRL);
    ]
);

impl_wrap!(
    [WrapLocal],

    ContentProtectionReg as u64,

    enum get_set [
        cp(CP);
    ],

    int get_set [
        ur_tag(UR_TAG);
    ],

    bool get_set [
        is_ready(READY);
        is_ces(CES);
    ]
);

impl_wrap!(
    [WrapLocal],

    AudioSamplePacketChannelMappingReg as u64,

    int get_set [
        asp_slot(ASP_SLOT);
        converter_channel(CONVERTER_CHANNEL);
    ]
);

#[repr(u32)]
pub enum GetParameterParam {
    VendorID = 0x0,
    RevisionID = 0x2,
    NodeCount = 0x4,
    FunctionGroupType = 0x5,
    AudioFunctionGroupCap = 0x8,
    AudioWidgetCap = 0x9,
    SupportedPCMRates = 0xA,
    SupportedStreamFormats = 0xB,
    PinCap = 0xC,
    InputAmplifierCap = 0xD,
    OutputAmplifierCap = 0x12,
    ConnectionListLength = 0xE,
    SupportedPowerStates = 0xF,
    ProcessingCap = 0x10,
    GPIOCount = 0x11,
    VolumeKnobCap = 0x13,
}

pub trait Command {}

pub trait NodeCommand: Command {
    const COMMAND: u32;
    const PAYLOAD_SIZE: usize;
    type Data: Into<u32>;
    type Output: From<u64>;
}

pub trait NodeCommandConstData: Command {
    const COMMAND: u32;
    const PAYLOAD_SIZE: usize;
    const DATA: u32;
    type Output: From<u64>;
}

macro_rules! impl_nodecommand (
    ($name:ident, $command:expr $(, p:$payload:expr)? $(, d:$data:ty)? $(, o:$output:ty)?) => (
        pub struct $name;
        impl Command for $name {}
        impl NodeCommand for $name {
            const COMMAND: u32 = $command;
            const PAYLOAD_SIZE: usize = or_default!(expr 8 $(, $payload)?);
            type Data = or_default!(type u32 $(, $data)?);
            type Output = or_default!(type u64 $(, $output)?);
        }
    );
);
macro_rules! impl_const_nodecommand (
    ($name:ident, $command:expr $(, p:$payload:expr)? $(, d:$data:expr)? $(, o:$output:ty)?) => (
        pub struct $name;
        impl Command for $name {}
        impl NodeCommandConstData for $name {
            const COMMAND: u32 = $command;
            const PAYLOAD_SIZE: usize = or_default!(expr 8 $(, $payload)?);
            const DATA: u32 = or_default!(expr 0 $(, $data as u32)?);
            type Output = or_default!(type u64 $(, $output)?);
        }
    );
);

macro_rules! impl_nodes {
    (const $name:ident($command:expr $(, p:$payload:expr)? $(, d:$data:expr)? $(, o:$output:ident)?)) => {
        impl_const_nodecommand!($name, $command $(, p:$payload)? $(, d:$data)? $(, o:WrapLocal<u64, $output::Register>)?
        );
    };
    (data $name:ident($command:expr $(, p:$payload:expr)? $(, d:$data:ident)? $(, o:$output:ident)?)) => {
        impl_nodecommand!($name, $command $(, p:$payload)? $(, d:WrapLocal<u64, $data::Register>)? $(, o:WrapLocal<u64, $output::Register>)?
        );
    };
    (param $name:ident()) => {
        paste::paste!(
            impl_nodes!(
                const [<GetParameter $name>](0xF00, d:GetParameterParam::$name, o:[<GetParameter $name Reg>])
            );
        );
    };
    ($($class:ident $name:ident($($t:tt)*);)*) => {
        $(
            impl_nodes!($class $name($($t)*));
        )*
    }
}

impl_nodes! {
    param VendorID();
    param RevisionID();
    param NodeCount();
    param FunctionGroupType();
    param AudioFunctionGroupCap();
    param AudioWidgetCap();
    param SupportedPCMRates();
    param SupportedStreamFormats();
    param PinCap();
    param InputAmplifierCap();
    param OutputAmplifierCap();
    param ConnectionListLength();
    param SupportedPowerStates();
    param ProcessingCap();
    param GPIOCount();
    param VolumeKnobCap();

    const GetConnectionSelectionControl(0xF01);
    data  SetConnectionSelectionControl(0x701);

    data  GetConnectionListEntry(0xF02, o:ConnectionListEntryReg);

    const GetProcessingState(0xF03, o:ProcessingStateReg);
    data  SetProcessingState(0x703, o:ProcessingStateReg);

    const GetCoefficientIndex(0xD, p:16);
    data  SetCoefficientIndex(0x5, p:16);

    const GetProcessingCoefficient(0xC, p:16);
    data  SetProcessingCoefficient(0x4, p:16);

    data  GetAmplifierGainMute(0xB, p:16, d:GetAmplifierGainMutePayloadReg,
                                          o:GetAmplifierGainMuteResponseReg);
    data  SetAmplifierGainMute(0x3, p:16, d:SetAmplifierGainMutePayloadReg);

    const GetConverterFormat(0xA, p:16, o:StreamFormatReg);
    data  SetConverterFormat(0x2, p:16, d:StreamFormatReg);

    const GetSPDIFControl (0xF0D, o:SPDIFControlReg);
    data  SetSPDIFControl1(0x70D);
    data  SetSPDIFControl2(0x70E);
    data  SetSPDIFControl3(0x73E);
    data  SetSPDIFControl4(0x73F);

    const GetPowerState(0xF05, o:PowerStateReg);
    data  SetPowerState(0x705, d:PowerStateReg);

    const GetChannelStreamID(0xF06, o:ChannelStreamIDReg);
    data  SetChannelStreamID(0x706, d:ChannelStreamIDReg);

    const GetSDISelect(0xF04);
    data  SetSDISelect(0x704);

    const GetPinWidgetControl(0xF07, o:PinWidgetControlReg);
    data  SetPinWidgetControl(0x707, d:PinWidgetControlReg);

    const GetUnsolicitedResponse(0xF08, o:EnableUnsolReg);
    data  SetUnsolicitedResponse(0x708, d:EnableUnsolReg);

    const GetPinSense(0xF09, o:PinSenseReg);

    const GetEAPDBTLEnable(0xF0C, o:EAPDBTLEnableReg);
    data  SetEAPDBTLEnable(0x70C, d:EAPDBTLEnableReg);

    const GetGPIData(0xF10);
    data  SetGPIData(0x710);

    const GetGPIWakeEnableMask(0xF11);
    data  SetGPIWakeEnableMask(0x711);

    const GetGPIUnsolicitedEnableMask(0xF12);
    data  SetGPIUnsolicitedEnableMask(0x712);

    const GetGPIStickyMask(0xF13);
    data  SetGPIStickyMask(0x713);

    const GetGPOData(0xF14);
    data  SetGPOData(0x714);

    const GetGPIOData(0xF15);
    data  SetGPIOData(0x715);

    const GetGPIOEnableMask(0xF16);
    data  SetGPIOEnableMask(0x716);

    const GetGPIODirection(0xF17);
    data  SetGPIODirection(0x717);

    const GetGPIOWakeEnableMask(0xF18);
    data  SetGPIOWakeEnableMask(0x718);

    const GetGPIOUnsolicitedEnableMask(0xF19);
    data  SetGPIOUnsolicitedEnableMask(0x719);

    const GetGPIOStickyMask(0xF1A);
    data  SetGPIOStickyMask(0x71A);

    const GetBeepGeneration(0xF0A);
    data  SetBeepGeneration(0x70A);

    const GetVolumeKnob(0xF0F, o:VolumeKnobReg);
    data  SetVolumeKnob(0x70F, d:VolumeKnobReg);

    const SetFunctionReset(0x7FF);

    const GetImplementationID (0xF20, o:ImplementationIDReg);
    data  SetImplementationID1(0x720);
    data  SetImplementationID2(0x721);
    data  SetImplementationID3(0x722);
    data  SetImplementationID4(0x723);

    const GetConfigurationDefault (0xF1C, o:ConfigurationDefaultReg);
    data  SetConfigurationDefault1(0x71C);
    data  SetConfigurationDefault2(0x71D);
    data  SetConfigurationDefault3(0x71E);
    data  SetConfigurationDefault4(0x71F);

    const GetStripeControl(0xF24, o:StripeControlReg);
    data  SetStripeControl(0x724, d:StripeControlReg);

    const GetConverterChannelCount(0xF2D);
    data  SetConverterChannelCount(0x72D);

    data  GetDataIslandPacketSizeInfo(0xF2E, d:DataIslandPacketSizePayloadReg);
    const GetDataIslandPacketIndex(   0xF30, o:DataIslandPacketIndexReg);
    data  SetDataIslandPacketIndex(   0x730, d:DataIslandPacketIndexReg);
    const GetDataIslandPacketData(    0xF31);
    data  SetDataIslandPacketData(    0x731);
    const GetDataIslandPacketXmitCtrl(0xF32, o:DataIslandPacketTXCtrl);
    data  SetDataIslandPacketXmitCtrl(0xF32, d:DataIslandPacketTXCtrl);

    const GetContentProtection(0xF33, o:ContentProtectionReg);
    data  SetContentProtection(0x733, d:ContentProtectionReg);

    data  GetAudioSamplePacketMapping(0xF34, d:AudioSamplePacketChannelMappingReg,
                                             o:AudioSamplePacketChannelMappingReg);
    data  SetAudioSamplePacketMapping(0x734, d:AudioSamplePacketChannelMappingReg);
}
