#[repr(u8)]
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub enum AtaCommand {
    AtaCommandWriteDma = 0xCA,
    AtaCommandWriteDmaQueued = 0xCC,
    AtaCommandWriteMultiple = 0xC5,
    AtaCommandWriteSectors = 0x30,

    AtaCommandReadDma = 0xC8,
    AtaCommandReadDmaQueued = 0xC7,
    AtaCommandReadMultiple = 0xC4,
    AtaCommandReadSectors = 0x20,

    AtaCommandWriteDmaExt = 0x35,
    AtaCommandWriteDmaQueuedExt = 0x36,
    AtaCommandWriteMultipleExt = 0x39,
    AtaCommandWriteSectorsExt = 0x34,

    AtaCommandReadDmaExt = 0x25,
    AtaCommandReadDmaQueuedExt = 0x26,
    AtaCommandReadMultipleExt = 0x29,
    AtaCommandReadSectorsExt = 0x24,

    AtaCommandPacket = 0xA0,
    AtaCommandDeviceReset = 0x08,

    AtaCommandService = 0xA2,
    AtaCommandNop = 0,
    AtaCommandNopNopAutopoll = 1,

    AtaCommandGetMediaStatus = 0xDA,

    AtaCommandFlushCache = 0xE7,
    AtaCommandFlushCacheExt = 0xEA,

    AtaCommandDataSetManagement = 0x06,

    AtaCommandMediaEject = 0xED,

    AtaCommandIdentifyPacketDevice = 0xA1,
    AtaCommandIdentifyDevice = 0xEC,

    AtaCommandSetFeatures = 0xEF,
    AtaCommandSetFeaturesEnableReleaseInt = 0x5D,
    AtaCommandSetFeaturesEnableServiceInt = 0x5E,
    AtaCommandSetFeaturesDisableReleaseInt = 0xDD,
    AtaCommandSetFeaturesDisableServiceInt = 0xDE,
}
