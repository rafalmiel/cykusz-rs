#![no_std]

#[macro_use]
extern crate bitflags;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_CHDIR: usize = 4;
pub const SYS_GETCWD: usize = 5;
pub const SYS_MKDIR: usize = 6;
pub const SYS_GETDENTS: usize = 7;
pub const SYS_EXIT: usize = 8;
pub const SYS_SLEEP: usize = 9;
pub const SYS_POWEROFF: usize = 10;
pub const SYS_REBOOT: usize = 11;
pub const SYS_GETADDRINFO: usize = 12;
pub const SYS_BIND: usize = 13;
pub const SYS_CONNECT: usize = 14;
pub const SYS_SELECT: usize = 15;
pub const SYS_MOUNT: usize = 16;
pub const SYS_UMOUNT: usize = 17;
pub const SYS_TIME: usize = 18;
pub const SYS_SYMLINK: usize = 19;
pub const SYS_RMDIR: usize = 20;
pub const SYS_UNLINK: usize = 21;
pub const SYS_LINK: usize = 22;
pub const SYS_RENAME: usize = 23;
pub const SYS_FORK: usize = 24;
pub const SYS_EXEC: usize = 25;
pub const SYS_FCNTL: usize = 26;
pub const SYS_MMAP: usize = 27;

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

bitflags! {
    pub struct OpenFlags: usize {
        const RDONLY      = 1 << 0;
        const WRONLY      = 1 << 1;
        const RDWR        = 1 << 2;
        const CREAT       = 0o100;
        const DIRECTORY   = 0o200;
    }
}

bitflags! {
    pub struct ConnectionFlags: usize {
        const UDP   = (1usize << 0);
        const TCP   = (1usize << 1);
    }
}

#[repr(i64)]
pub enum FcntlCmd {
    GetFL = 1,
    Inval = -1,
}

impl From<u64> for FcntlCmd {
    fn from(v: u64) -> Self {
        match v {
            1 => FcntlCmd::GetFL,
            _ => FcntlCmd::Inval,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum FileType {
    File = 0x1,
    Dir = 0x2,
    DevNode = 0x3,
    Symlink = 0x4,
}

impl Default for FileType {
    fn default() -> FileType {
        FileType::File
    }
}

#[repr(C)]
pub struct SysDirEntry {
    pub ino: usize,
    pub off: usize,
    pub reclen: usize,
    pub typ: FileType,
    pub name: [u8; 0],
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
