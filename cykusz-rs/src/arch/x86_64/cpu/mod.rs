pub mod fpu;

use crate::arch::raw::ctrlregs;
use crate::arch::raw::ctrlregs::{Cr0, Cr4, XCr0};
use raw_cpuid::CpuId;

pub fn has_x2apic() -> bool {
    CpuId::new()
        .get_feature_info()
        .map_or(false, |f| f.has_x2apic())
}

fn enable_nxe_bit() {
    use crate::arch::raw::msr::{rdmsr, wrmsr, IA32_EFER};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

fn enable_write_protect_bit() {
    use crate::arch::raw::ctrlregs::{cr0, cr0_write, Cr0};

    unsafe { cr0_write(cr0() | Cr0::CR0_WRITE_PROTECT) };
}

fn enable_sse() -> bool {
    if !CpuId::new()
        .get_feature_info()
        .map_or(false, |f| f.has_sse() && f.has_fxsave_fxstor())
    {
        return false;
    }

    unsafe {
        let mut cr0 = ctrlregs::cr0();
        cr0.remove(Cr0::CR0_EMULATE_COPROCESSOR);
        cr0.insert(Cr0::CR0_MONITOR_COPROCESSOR);
        ctrlregs::cr0_write(cr0);
    }

    unsafe {
        let mut cr4 = ctrlregs::cr4();
        cr4.insert(Cr4::CR4_ENABLE_SSE);
        cr4.insert(Cr4::CR4_UNMASKED_SSE);
        ctrlregs::cr4_write(cr4);
    }

    true
}

fn enable_avx() {
    if !CpuId::new()
        .get_feature_info()
        .map_or(false, |f| f.has_avx() && f.has_xsave())
    {
        dbgln!(cpu, "avx not enabled");
        return;
    }

    let ext_info = if let Some(f) = CpuId::new().get_extended_state_info() {
        f
    } else {
        dbgln!(cpu, "avx not enabled 2");
        return;
    };

    unsafe {
        let mut cr4 = ctrlregs::cr4();
        cr4.insert(Cr4::CR4_ENABLE_OS_XSAVE);
        ctrlregs::cr4_write(cr4);
    }

    unsafe {
        let mut xcr0 = ctrlregs::xcr0_read();
        assert!(xcr0.contains(XCr0::XCR0_X87));

        if ext_info.xcr0_supports_sse_128() {
            xcr0.insert(XCr0::XCR0_SSE);
        }
        if ext_info.xcr0_supports_avx_256() {
            xcr0.insert(XCr0::XCR0_AVX);
        }
        //if ext_info.xcr0_supports_mpx_bndregs() {
        //    xcr0.insert(XCr0::XCR0_BNDREG);
        //}
        //if ext_info.xcr0_supports_mpx_bndcsr() {
        //    xcr0.insert(XCr0::XCR0_BNDCSR);
        //}
        //if ext_info.xcr0_supports_avx512_opmask() {
        //    xcr0.insert(XCr0::XCR0_OPMASK);
        //}
        //if ext_info.xcr0_supports_avx512_zmm_hi256() {
        //    xcr0.insert(XCr0::XCR0_ZMM_HI256);
        //}
        //if ext_info.xcr0_supports_avx512_zmm_hi16() {
        //    xcr0.insert(XCr0::XCR0_HI16_ZMM);
        //}
        //if ext_info.xcr0_supports_pkru() {
        //    xcr0.insert(XCr0::XCR0_PKRU_STATE);
        //}

        ctrlregs::xcr0_write(xcr0);
        dbgln!(cpu, "AVX enabled");
    }
}

pub fn init() {
    enable_nxe_bit();
    enable_write_protect_bit();
    if enable_sse() {
        enable_avx();
    }
    fpu::init();
}
