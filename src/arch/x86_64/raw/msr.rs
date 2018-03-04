/// Write 64 bits to msr register.
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!("wrmsr" :: "{ecx}" (msr), "{eax}" (low), "{edx}" (high) : "memory" : "volatile" );
}

/// Read 64 bits msr register.
#[allow(unused_mut)]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    asm!("rdmsr" : "={eax}" (low), "={edx}" (high) : "{ecx}" (msr) : "memory" : "volatile");
    ((high as u64) << 32) | (low as u64)
}

/// If (  CPUID.80000001.EDX.[bit  20] or  CPUID.80000001.EDX.[bit 29])
pub const IA32_EFER: u32 = 0xc0000080;