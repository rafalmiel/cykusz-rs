use crate::arch::raw::ctrlregs::{cr4, Cr4};
use core::arch::x86_64::{_fxrstor64, _fxsave64, _xrstor64, _xsave64};
use spin::Once;

struct FpuInfo {
    has_fxsave: bool,
    has_xsave: bool,
}

impl FpuInfo {
    pub fn has_fxsave(&self) -> bool {
        self.has_fxsave
    }

    pub fn has_xsave(&self) -> bool {
        self.has_xsave
    }
}

static FPU_INFO: Once<FpuInfo> = Once::new();

fn fpu_info() -> &'static FpuInfo {
    unsafe { FPU_INFO.get_unchecked() }
}

#[repr(align(64))]
#[derive(Debug, Copy, Clone)]
pub struct FpuState([u8; 1024]);

impl Default for FpuState {
    fn default() -> Self {
        FpuState([0u8; 1024])
    }
}

impl FpuState {
    pub unsafe fn save(&mut self) {
        let info = fpu_info();
        if info.has_xsave() {
            _xsave64(self.0.as_mut_ptr(), u64::MAX);
        } else if info.has_fxsave() {
            _fxsave64(self.0.as_mut_ptr());
        }
    }

    pub unsafe fn restore(&self) {
        let info = fpu_info();
        if info.has_xsave() {
            _xrstor64(self.0.as_ptr(), u64::MAX);
        } else if info.has_fxsave() {
            _fxrstor64(self.0.as_ptr());
        }
    }
}

pub fn init() {
    FPU_INFO.call_once(|| {
        let cr4 = unsafe { cr4() };
        FpuInfo {
            has_fxsave: cr4.contains(Cr4::CR4_ENABLE_SSE),
            has_xsave: cr4.contains(Cr4::CR4_ENABLE_OS_XSAVE),
        }
    });
}
