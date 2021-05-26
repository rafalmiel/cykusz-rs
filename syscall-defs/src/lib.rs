#![feature(new_uninit)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate bitflags;

use core::convert::TryFrom;

pub mod exec;
pub mod ioctl;
pub mod prctl;
pub mod resource;
pub mod signal;
pub mod stat;
pub mod time;

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
pub const SYS_MUNMAP: usize = 28;
pub const SYS_MAPS: usize = 29;

pub const SYS_SEEK: usize = 30;
pub const SYS_PREAD: usize = 31;
pub const SYS_PWRITE: usize = 32;

pub const SYS_WAITPID: usize = 33;
pub const SYS_IOCTL: usize = 34;
pub const SYS_SIGACTION: usize = 35;
pub const SYS_SIGRETURN: usize = 36;
pub const SYS_SIGPROCMASK: usize = 37;

pub const SYS_FUTEX_WAIT: usize = 38;
pub const SYS_FUTEX_WAKE: usize = 39;

pub const SYS_ARCH_PRCTL: usize = 40;
pub const SYS_SPAWN_THREAD: usize = 41;
pub const SYS_EXIT_THREAD: usize = 42;
pub const SYS_GETPID: usize = 43;
pub const SYS_GETTID: usize = 44;
pub const SYS_SETSID: usize = 45;
pub const SYS_SETPGID: usize = 46;
pub const SYS_PIPE: usize = 47;
pub const SYS_DUP: usize = 48;
pub const SYS_DUP2: usize = 49;
pub const SYS_STAT: usize = 50;
pub const SYS_FSTAT: usize = 51;

pub const SYS_GETRLIMIT: usize = 52;
pub const SYS_DEBUG: usize = 53;

pub const SYS_ACCESS: usize = 54;

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u64)]
pub enum SyscallError {
    EDOM = 1,
    EILSEQ = 2,
    ERANGE = 3,

    E2BIG = 1001,
    EACCES = 1002,
    EADDRINUSE = 1003,
    EADDRNOTAVAIL = 1004,
    EAFNOSUPPORT = 1005,
    EAGAIN = 1006,
    EALREADY = 1007,
    EBADF = 1008,
    EBADMSG = 1009,
    EBUSY = 1010,
    ECANCELED = 1011,
    ECHILD = 1012,
    ECONNABORTED = 1013,
    ECONNREFUSED = 1014,
    ECONNRESET = 1015,
    EDEADLK = 1016,
    EDESTADDRREQ = 1017,
    EDQUOT = 1018,
    EEXIST = 1019,
    EFAULT = 1020,
    EFBIG = 1021,
    EHOSTUNREACH = 1022,
    EIDRM = 1023,
    EINPROGRESS = 1024,
    EINTR = 1025,
    EINVAL = 1026,
    EIO = 1027,
    EISCONN = 1028,
    EISDIR = 1029,
    ELOOP = 1030,
    EMFILE = 1031,
    EMLINK = 1032,
    EMSGSIZE = 1034,
    EMULTIHOP = 1035,
    ENAMETOOLONG = 1036,
    ENETDOWN = 1037,
    ENETRESET = 1038,
    ENETUNREACH = 1039,
    ENFILE = 1040,
    ENOBUFS = 1041,
    ENODEV = 1042,
    ENOENT = 1043,
    ENOEXEC = 1044,
    ENOLCK = 1045,
    ENOLINK = 1046,
    ENOMEM = 1047,
    ENOMSG = 1048,
    ENOPROTOOPT = 1049,
    ENOSPC = 1050,
    ENOSYS = 1051,
    ENOTCONN = 1052,
    ENOTDIR = 1053,
    ENOTEMPTY = 1054,
    ENOTRECOVERABLE = 1055,
    ENOTSOCK = 1056,
    ENOTSUP = 1057,
    ENOTTY = 1058,
    ENXIO = 1059,
    EOPNOTSUPP = 1060,
    EOVERFLOW = 1061,
    EOWNERDEAD = 1062,
    EPERM = 1063,
    EPIPE = 1064,
    EPROTO = 1065,
    EPROTONOSUPPORT = 1066,
    EPROTOTYPE = 1067,
    EROFS = 1068,
    ESPIPE = 1069,
    ESRCH = 1070,
    ESTALE = 1071,
    ETIMEDOUT = 1072,
    ETXTBSY = 1073,
    EXDEV = 1075,
    ENODATA = 1076,
    ETIME = 1077,
    ENOKEY = 1078,
    ESHUTDOWN = 1079,
    EHOSTDOWN = 1080,
    EBADFD = 1081,
    ENOMEDIUM = 1082,
    ENOTBLK = 1083,

    UnknownError = 0xffff,
}

pub enum OpenFD {
    Fd(usize),
    Cwd,
}

impl TryFrom<u64> for OpenFD {
    type Error = SyscallError;

    fn try_from(v: u64) -> Result<OpenFD, SyscallError> {
        let v = v as isize;

        match v {
            -100 => Ok(OpenFD::Cwd),
            a if a >= 0 && a < 256 => Ok(OpenFD::Fd(a as usize)),
            _ => Err(SyscallError::EINVAL),
        }
    }
}

impl From<OpenFD> for usize {
    fn from(v: OpenFD) -> Self {
        match v {
            OpenFD::Fd(a) => a,
            OpenFD::Cwd => (-100isize) as usize,
        }
    }
}

bitflags! {
    pub struct OpenFlags: usize {
        const RDONLY      = 2;
        const RDWR        = 3;
        const WRONLY      = 5;
        const CREAT       = 0x10;
        const DIRECTORY   = 0x20;
        const EXCL        = 0x40;
        const NOCTTY      = 0x80;
        const TRUNC       = 0x0200;
        const CLOEXEC     = 0x4000;
    }
}

#[repr(usize)]
#[derive(Debug)]
pub enum SeekWhence {
    SeekSet = 0,
    SeekCur = 1,
    SeekEnd = 2,
}

impl From<u64> for SeekWhence {
    fn from(v: u64) -> Self {
        match v {
            0 => SeekWhence::SeekSet,
            1 => SeekWhence::SeekCur,
            2 => SeekWhence::SeekEnd,
            _ => panic!("Invalid SeekWhence {}", v),
        }
    }
}

impl From<SeekWhence> for usize {
    fn from(s: SeekWhence) -> Self {
        s as u64 as usize
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

bitflags! {
    pub struct MMapProt: usize {
        const PROT_READ = 0x1;
        const PROT_WRITE = 0x2;
        const PROT_EXEC = 0x4;
        const PROT_NONE = 0x0;
    }
}

bitflags! {
    pub struct MMapFlags: usize {
        const MAP_PRIVATE = 0x1;
        const MAP_SHARED = 0x2;
        const MAP_FIXED = 0x4;
        const MAP_ANONYOMUS = 0x8;
    }
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
            let e: SyscallError = unsafe { core::mem::transmute((-e) as u64) };

            Err(e)
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
