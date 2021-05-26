use crate::SyscallError;
use core::convert::TryFrom;

pub enum RLimitKind {
    Core = 1,
    Cpu = 2,
    Data = 3,
    FSize = 4,
    NOFile = 5,
    Stack = 6,
    As = 7,
    Memlock = 8,
    Rss = 9,
    NProc = 10,
    Locks = 11,
    SigPending = 12,
    MsgQueue = 13,
    Nice = 14,
    RTPrio = 15,
    NLimits = 16,
}

#[repr(C)]
pub struct RLimit {
    pub cur: u64,
    pub max: u64,
}

impl TryFrom<u64> for RLimitKind {
    type Error = SyscallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RLimitKind::Core),
            2 => Ok(RLimitKind::Cpu),
            3 => Ok(RLimitKind::Data),
            4 => Ok(RLimitKind::FSize),
            5 => Ok(RLimitKind::NOFile),
            6 => Ok(RLimitKind::Stack),
            7 => Ok(RLimitKind::As),
            8 => Ok(RLimitKind::Memlock),
            9 => Ok(RLimitKind::Rss),
            10 => Ok(RLimitKind::NProc),
            11 => Ok(RLimitKind::Locks),
            12 => Ok(RLimitKind::SigPending),
            13 => Ok(RLimitKind::MsgQueue),
            14 => Ok(RLimitKind::Nice),
            15 => Ok(RLimitKind::RTPrio),
            _ => Err(SyscallError::EINVAL),
        }
    }
}
