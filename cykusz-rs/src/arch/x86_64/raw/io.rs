use core::arch::asm;

/// Write 8 bits to port
pub unsafe fn outb(port: u16, val: u8) { unsafe {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
    );
}}

/// Read 8 bits from port
pub unsafe fn inb(port: u16) -> u8 { unsafe {
    let ret: u8;
    asm!("in al, dx", out("al") ret, in("dx") port);
    ret
}}

/// Write 16 bits to port
pub unsafe fn outw(port: u16, val: u16) { unsafe {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") val,
    );
}}

/// Read 16 bits from port
pub unsafe fn inw(port: u16) -> u16 { unsafe {
    let ret: u16;
    asm!("in ax, dx", out("ax") ret, in("dx") port);
    ret
}}

/// Write 32 bits to port
pub unsafe fn outl(port: u16, val: u32) { unsafe {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") val,
    );
}}

/// Read 32 bits from port
pub unsafe fn inl(port: u16) -> u32 { unsafe {
    let ret: u32;
    asm!("in eax, dx", out("eax") ret, in("edx") port);
    ret
}}
