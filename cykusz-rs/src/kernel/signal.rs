use crate::kernel::fs::vfs::FsError;
use syscall_defs::SyscallError;

#[derive(Debug, PartialEq)]
pub enum SignalError {
    Interrupted,
}

pub type SignalResult<T> = core::result::Result<T, SignalError>;

impl From<SignalError> for FsError {
    fn from(s: SignalError) -> Self {
        match s {
            SignalError::Interrupted => FsError::Interrupted,
        }
    }
}

impl From<SignalError> for SyscallError {
    fn from(s: SignalError) -> Self {
        match s {
            SignalError::Interrupted => SyscallError::Interrupted,
        }
    }
}
