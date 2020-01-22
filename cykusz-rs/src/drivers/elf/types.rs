#![allow(dead_code)]

#[repr(u8)]
#[derive(Debug)]
pub enum Class {
    Bit32 = 1,
    Bit64 = 2,
}

#[repr(u8)]
#[derive(Debug)]
pub enum Endianess {
    Little = 1,
    Big = 2,
}

#[repr(u8)]
#[derive(Debug)]
pub enum Abi {
    SysV = 0,
    HPUS = 1,
    NetBSD = 2,
    GNU = 3,
    Solaris = 6,
    AIX = 7,
    Irix = 8,
    FreeBSD = 9,
    Tru64 = 10,
    Modesto = 11,
    OpenBSD = 12,
    ArmAeabi = 64,
    Arm = 97,
    Standalone = 255,
}

#[repr(u16)]
#[derive(Debug)]
pub enum BinType {
    None = 0,
    Rel = 1,
    Exec = 2,
    Dyn = 3,
    Core = 4,
    LoOS = 0xfe00,
    HiOS = 0xfeff,
    LoProc = 0xff00,
    HiProc = 0xffff,
}

#[repr(u16)]
#[derive(Debug)]
pub enum Machine {
    None = 0x00,
    Sparc = 0x02,
    X86 = 0x03,
    Mips = 0x08,
    PowerPC = 0x14,
    S390 = 0x16,
    Arm = 0x28,
    SuperH = 0x2A,
    IA64 = 0x32,
    X8664 = 0x3E,
    AArch64 = 0xB7,
    RiscV = 0xF3,
}

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum ProgramType {
    Null = 0x0,
    Load = 0x1,
    Dynamic = 0x2,
    Interp = 0x3,
    Note = 0x4,
    ShLib = 0x5,
    PHDR = 0x6,
    TLS = 0x7,
    LoOS = 0x60000000,
    HiOS = 0x6FFFFFFF,
    LoProc = 0x70000000,
    HiProc = 0x7FFFFFFF,
}

bitflags!(
    pub struct ProgramFlags: u32 {
        const EXECUTABLE = 1;
        const WRITABLE = 2;
        const READABLE = 4;
    }
);
