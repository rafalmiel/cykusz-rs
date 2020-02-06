#![no_std]

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SyscallError {
    Perm = 1,
    NoEnt = 2,
    IO = 5,
    BadFD = 9,
    Again = 11,
    NoMem = 12,
    Access = 13,
    Fault = 14,
    Busy = 16,
    Exists = 17,
    NoDev = 19,
    NotDir = 20,
    IsDir = 21,
    Inval = 22,
    UnknownError = 0xffff,
}

pub type SyscallResult = Result<usize, SyscallError>;

pub trait SyscallFrom<T> {
    fn syscall_from(e: T) -> Self;
}

pub trait SyscallInto<T> {
    fn syscall_into(self) -> T;
}

impl SyscallFrom<isize> for SyscallResult {
    fn syscall_from(e: isize) -> Self {
        if e >= 0 {
            return Ok(e as usize);
        } else {
            match e {
                -1 => Err(SyscallError::Perm),
                -2 => Err(SyscallError::NoEnt),
                -5 => Err(SyscallError::IO),
                -9 => Err(SyscallError::BadFD),
                -11 => Err(SyscallError::Again),
                -12 => Err(SyscallError::NoMem),
                -13 => Err(SyscallError::Access),
                -14 => Err(SyscallError::Fault),
                -16 => Err(SyscallError::Busy),
                -17 => Err(SyscallError::Exists),
                -19 => Err(SyscallError::NoDev),
                -20 => Err(SyscallError::NotDir),
                -21 => Err(SyscallError::IsDir),
                -22 => Err(SyscallError::Inval),
                _ => Err(SyscallError::UnknownError),
            }
        }
    }
}

impl SyscallInto<isize> for SyscallResult {
    fn syscall_into(self) -> isize {
        match self {
            Ok(v) => v as isize,
            Err(v) => -(v as isize),
        }
    }
}
