#![feature(slice_as_chunks)]
#![feature(int_roundings)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate bitflags;

use crate::net::MsgFlags;
use crate::stat::Mode;

pub mod events;
pub mod exec;
pub mod ioctl;
pub mod net;
pub mod poll;
pub mod prctl;
pub mod resource;
pub mod signal;
pub mod stat;
pub mod time;
pub mod waitpid;

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
pub const SYS_READLINK: usize = 51;

pub const SYS_GETRLIMIT: usize = 52;
pub const SYS_DEBUG: usize = 53;

pub const SYS_ACCESS: usize = 54;
pub const SYS_KILL: usize = 55;
pub const SYS_SYNC: usize = 56;
pub const SYS_FSYNC: usize = 57;
pub const SYS_TICKSNS: usize = 58;

pub const SYS_GETPPID: usize = 59;
pub const SYS_GETPGID: usize = 60;
pub const SYS_TRUNCATE: usize = 61;
pub const SYS_POLL: usize = 62;

pub const SYS_SOCKET: usize = 63;
pub const SYS_ACCEPT: usize = 64;
pub const SYS_LISTEN: usize = 65;
pub const SYS_MSGRECV: usize = 66;
pub const SYS_MSGSEND: usize = 67;
pub const SYS_SETSOCKOPT: usize = 68;
pub const SYS_GETSOCKOPT: usize = 69;
pub const SYS_YIELD: usize = 70;
pub const SYS_CHMOD: usize = 71;
pub const SYS_UTIME: usize = 72;
pub const SYS_MKNODE: usize = 73;
pub const SYS_SOCKETPAIR: usize = 74;
pub const SYS_MPROTECT: usize = 75;

pub const SYSCALL_STRING: [&'static str; 76] = [
    "SYS_READ",
    "SYS_WRITE",
    "SYS_OPEN",
    "SYS_CLOSE",
    "SYS_CHDIR",
    "SYS_GETCWD",
    "SYS_MKDIR",
    "SYS_GETDENTS",
    "SYS_EXIT",
    "SYS_SLEEP",
    "SYS_POWEROFF",
    "SYS_REBOOT",
    "SYS_GETADDRINFO",
    "SYS_BIND",
    "SYS_CONNECT",
    "SYS_SELECT",
    "SYS_MOUNT",
    "SYS_UMOUNT",
    "SYS_TIME",
    "SYS_SYMLINK",
    "SYS_RMDIR",
    "SYS_UNLINK",
    "SYS_LINK",
    "SYS_RENAME",
    "SYS_FORK",
    "SYS_EXEC",
    "SYS_FCNTL",
    "SYS_MMAP",
    "SYS_MUNMAP",
    "SYS_MAPS",
    "SYS_SEEK",
    "SYS_PREAD",
    "SYS_PWRITE",
    "SYS_WAITPID",
    "SYS_IOCTL",
    "SYS_SIGACTION",
    "SYS_SIGRETURN",
    "SYS_SIGPROCMASK",
    "SYS_FUTEX_WAIT",
    "SYS_FUTEX_WAKE",
    "SYS_ARCH_PRCTL",
    "SYS_SPAWN_THREAD",
    "SYS_EXIT_THREAD",
    "SYS_GETPID",
    "SYS_GETTID",
    "SYS_SETSID",
    "SYS_SETPGID",
    "SYS_PIPE",
    "SYS_DUP",
    "SYS_DUP2",
    "SYS_STAT",
    "SYS_READLINK",
    "SYS_GETRLIMIT",
    "SYS_DEBUG",
    "SYS_ACCESS",
    "SYS_KILL",
    "SYS_SYNC",
    "SYS_FSYNC",
    "SYS_TICKSNS",
    "SYS_GETPPID",
    "SYS_GETPGID",
    "SYS_TRUNCATE",
    "SYS_POLL",
    "SYS_SOCKET",
    "SYS_ACCEPT",
    "SYS_LISTEN",
    "SYS_MSGRECV",
    "SYS_MSGSEND",
    "SYS_SETSOCKOPT",
    "SYS_GETSOCKOPT",
    "SYS_YIELD",
    "SYS_CHMOD",
    "SYS_UTIME",
    "SYS_MKNODE",
    "SYS_SOCKETPAIR",
    "SYS_MPROTECT",
];

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
    ERESTART = 1106,

    ERESTARTSYS = 2000,
    ERESTARTNOHAND = 2001,
    ERESTARTNOINTR = 2002,

    UnknownError = 0xffff,
}

#[derive(Debug, Copy, Clone)]
pub enum OpenFD {
    Fd(usize),
    Cwd,
    None,
}

impl TryFrom<u64> for OpenFD {
    type Error = SyscallError;

    fn try_from(v: u64) -> Result<OpenFD, SyscallError> {
        let v = v as isize;

        match v {
            -100 => Ok(OpenFD::Cwd),
            -1 => Ok(OpenFD::None),
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
            OpenFD::None => (-1isize) as usize,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpenFlags: usize {
        const EXEC = 0o10000000;
        const RDONLY = 0o0;
        const WRONLY = 0o1;
        const RDWR = 0o2;
        const CREAT = 0o100;
        const EXCL = 0o200;
        const NOCTTY = 0o400;
        const TRUNC = 0o1000;
        const APPEND = 0o2000;
        const NONBLOCK = 0o4000;
        const DSYNC = 0o10000;
        const ASYNC = 0o20000;
        const DIRECT = 0o40000;
        const DIRECTORY = 0o200000;
        const NOFOLLOW = 0o400000;
        const CLOEXEC = 0o2000000;
        const SYNC = 0o4010000;
        const RSYNC = 0o4010000;
        const LARGEFILE = 0o100000;
        const NOATIME = 0o1000000;
        const TMPFILE = 0o20000000;
    }
}

impl From<MsgFlags> for OpenFlags {
    fn from(value: MsgFlags) -> Self {
        if value.contains(MsgFlags::MSG_DONTWAIT) {
            OpenFlags::NONBLOCK
        } else {
            OpenFlags::empty()
        }
    }
}

impl OpenFlags {
    pub fn set_fd_flags_mask() -> usize {
        (OpenFlags::APPEND
            | OpenFlags::ASYNC
            | OpenFlags::DIRECT
            | OpenFlags::NOATIME
            | OpenFlags::NONBLOCK)
            .bits()
    }

    pub fn is_open_mode(&self, open_mode: OpenFlags) -> bool {
        let mode_bits = self.bits() & 0b111usize;
        let req_bits = open_mode.bits() & 0b111usize;

        mode_bits == req_bits
    }

    pub fn is_readable(&self) -> bool {
        self.is_open_mode(OpenFlags::RDONLY) || self.is_open_mode(OpenFlags::RDWR)
    }

    pub fn is_writable(&self) -> bool {
        self.is_open_mode(OpenFlags::WRONLY) || self.is_open_mode(OpenFlags::RDWR)
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

#[repr(i64)]
#[derive(Debug)]
pub enum FcntlCmd {
    DupFD = 0,
    GetFD = 1,
    SetFD = 2,
    DupFDCloexec = 1030,
    GetFL = 3,
    SetFL = 4,
    Inval = -1,
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct FDFlags: u64 {
        const FD_CLOEXEC = 1;
    }
}

impl From<FDFlags> for OpenFlags {
    fn from(value: FDFlags) -> Self {
        let mut flags = OpenFlags::empty();

        if value.contains(FDFlags::FD_CLOEXEC) {
            flags.insert(OpenFlags::CLOEXEC);
        }

        flags
    }
}

impl From<OpenFlags> for FDFlags {
    fn from(value: OpenFlags) -> Self {
        let mut flags = FDFlags::empty();
        if value.contains(OpenFlags::CLOEXEC) {
            flags.insert(FDFlags::FD_CLOEXEC);
        }

        flags
    }
}

impl From<u64> for FcntlCmd {
    fn from(v: u64) -> Self {
        match v {
            0 => FcntlCmd::DupFD,
            1030 => FcntlCmd::DupFDCloexec,
            1 => FcntlCmd::GetFD,
            2 => FcntlCmd::SetFD,
            3 => FcntlCmd::GetFL,
            4 => FcntlCmd::SetFL,
            _ => FcntlCmd::Inval,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum FileType {
    Unknown = 0,
    Fifo = 1,
    Char = 2,
    Dir = 4,
    Block = 6,
    File = 8,
    Symlink = 10,
    Socket = 12,
}

impl Default for FileType {
    fn default() -> FileType {
        FileType::File
    }
}

impl From<Mode> for FileType {
    fn from(value: Mode) -> Self {
        match value.ftype_bits_truncate() {
            Mode::IFBLK => FileType::Block,
            Mode::IFCHR => FileType::Char,
            Mode::IFLNK => FileType::Symlink,
            Mode::IFDIR => FileType::Dir,
            Mode::IFREG => FileType::File,
            Mode::IFIFO => FileType::Fifo,
            Mode::IFSOCK => FileType::Socket,
            _ => panic!("Invalid mode {:?}", value.bits()),
        }
    }
}

#[repr(C)]
pub struct SysDirEntry {
    pub ino: usize,
    pub off: usize,
    pub reclen: u16,
    pub typ: FileType,
    pub name: [u8; 0],
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub struct MMapProt: usize {
        const PROT_READ = 0x1;
        const PROT_WRITE = 0x2;
        const PROT_EXEC = 0x4;
        const PROT_NONE = 0x0;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub struct MMapFlags: usize {
        const MAP_PRIVATE = 0x1;
        const MAP_SHARED = 0x2;
        const MAP_FIXED = 0x4;
        const MAP_ANONYOMUS = 0x8;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct AtFlags: u64 {
        const EMPTY_PATH = 0x1000;
        const SYMLINK_FOLLOW = 0x400;
        const SYMLINK_NOFOLLOW = 0x100;
        const REMOVEDIR = 0x200;
        const EACCESS = 0x200;
    }
}

pub type SyscallResult = Result<usize, SyscallError>;

pub trait SyscallRestartable {
    fn maybe_into_erestartsys(&self) -> SyscallResult;
    fn maybe_into_erestartnohand(&self) -> SyscallResult;
    fn maybe_into_erestartnointr(&self) -> SyscallResult;

    fn is_restart(&self, has_handler: bool, restart_flag: bool) -> bool;
}

impl SyscallRestartable for SyscallResult {
    fn maybe_into_erestartsys(&self) -> SyscallResult {
        if let Err(SyscallError::EINTR) = self {
            Err(SyscallError::ERESTARTSYS)
        } else {
            *self
        }
    }
    fn maybe_into_erestartnohand(&self) -> SyscallResult {
        if let Err(SyscallError::EINTR) = self {
            Err(SyscallError::ERESTARTNOHAND)
        } else {
            *self
        }
    }
    fn maybe_into_erestartnointr(&self) -> SyscallResult {
        if let Err(SyscallError::EINTR) = self {
            Err(SyscallError::ERESTARTNOINTR)
        } else {
            *self
        }
    }

    fn is_restart(&self, has_handler: bool, restart_flag: bool) -> bool {
        match self {
            Err(SyscallError::ERESTARTNOINTR) => true,
            Err(SyscallError::ERESTARTSYS) => !has_handler || (has_handler && restart_flag),
            Err(SyscallError::ERESTARTNOHAND) => !has_handler,
            _ => false,
        }
    }
}

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
            Err(SyscallError::ERESTARTNOHAND)
            | Err(SyscallError::ERESTARTNOINTR)
            | Err(SyscallError::ERESTARTSYS) => -(SyscallError::EINTR as isize),
            Err(v) => -(v as isize),
        }
    }
}
