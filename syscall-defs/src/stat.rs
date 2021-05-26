bitflags! {
    #[derive(Default)]
    pub struct Mode: u32 {
        const IFBLK = 0x06000;
        const IFCHR = 0x02000;
        const IFIFO = 0x01000;
        const IFREG = 0x08000;
        const IFDIR = 0x04000;
        const IFLNK = 0x0A000;
        const IFSOCK = 0x0C000;

        const IRWXU = 0o700;
        const IRUSR = 0o400;
        const IWUSR = 0o200;
        const IXUSR = 0o100;

        const IRWXG = 0o70;
        const IRGRP = 0o40;
        const IWGRP = 0o20;
        const IXGRP = 0o10;

        const IRWXO = 0o7;
        const IROTH = 0o4;
        const IWOTH = 0o2;
        const IXOTH = 0o1;

        const ISUID = 0o4000;
        const ISGID = 0o2000;
        const ISVTX = 0o1000;

        const IREAD = Mode::IRUSR.bits();
        const IWRITE = Mode::IWUSR.bits();
        const IEXEC = Mode::IXUSR.bits();
    }
}

#[repr(C)]
#[derive(Default, Debug)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: Mode,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_atim: crate::time::Timespec,
    pub st_mtim: crate::time::Timespec,
    pub st_ctim: crate::time::Timespec,
    pub st_blksize: u64,
    pub st_blocks: u64,
}
