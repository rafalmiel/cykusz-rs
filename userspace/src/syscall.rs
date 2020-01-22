pub unsafe fn syscall2(mut a: usize, b: usize, c: usize) -> usize {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    a
}

pub fn read(buf: *mut u8, len: usize) -> usize {
    unsafe {
        syscall2(1, buf as usize, len)
    }
}

pub fn print(v: &str) {
    unsafe {
        syscall2(0, v.as_ptr() as usize, v.len());
    }
}
