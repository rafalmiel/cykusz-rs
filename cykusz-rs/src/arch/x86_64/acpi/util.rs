pub unsafe fn checksum(addr: *const u8, size: isize) -> bool {
    let mut s: u32 = 0;

    for i in 0..size {
        s += *addr.offset(i) as u32;
    }

    (s & 0xFF) == 0
}
