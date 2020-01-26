pub unsafe fn syscall3(mut a: usize, b: usize, c: usize, d: usize) -> usize {
    asm!("syscall"
        : "={rax}"(a)
        : "{rax}"(a), "{rdi}"(b), "{rsi}"(c), "{rdx}"(d)
        : "memory", "rcx", "r11"
        : "intel", "volatile");

    a
}

pub fn read(fd: usize, buf: *mut u8, len: usize) -> usize {
    unsafe { syscall3(1, fd, buf as usize, len) }
}

pub fn print(v: &str) {
    unsafe {
        syscall3(0, 0, v.as_ptr() as usize, v.len());
    }
}
