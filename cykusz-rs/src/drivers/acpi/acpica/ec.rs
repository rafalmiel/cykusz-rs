use acpica::*;

use crate::arch::raw::cpuio::Port;
use crate::kernel::timer::busy_sleep;

struct GlobalLockGuard {
    handle: i32,
}

impl GlobalLockGuard {
    fn new() -> GlobalLockGuard {
        let mut lock = GlobalLockGuard { handle: 0 };

        assert_eq!(
            unsafe {
                acpica::AcpiAcquireGlobalLock(u16::max_value() as i16, &mut lock.handle as *mut i32)
            },
            acpica::AE_OK
        );

        lock
    }
}

impl Drop for GlobalLockGuard {
    fn drop(&mut self) {
        unsafe {
            acpica::AcpiReleaseGlobalLock(self.handle);
        }
    }
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
pub unsafe extern "C" fn embedded_ctl(
    Function: acpica::UINT32,
    Address: acpica::ACPI_PHYSICAL_ADDRESS,
    BitWidth: acpica::UINT32,
    Value: *mut acpica::UINT64,
    _HandlerContext: *mut ::core::ffi::c_void,
    _RegionContext: *mut ::core::ffi::c_void,
) -> acpica::ACPI_STATUS {
    let mut data = Port::<u8>::new(62);
    let mut cmd = Port::<u8>::new(66);

    let wait_for = |mask, value| -> bool {
        let mut cmd = Port::<u8>::new(66);
        for _ in 0..1000 {
            if cmd.read() & mask == value {
                return true;
            } else {
                busy_sleep(100000);
            }
        }

        //println!("EC: Wait Failed");
        return false;
    };

    let _lock = GlobalLockGuard::new();

    if BitWidth != 8 {
        panic!("Unsupported BitWidth {}", BitWidth);
    }

    if Function == 0 {
        //Read
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            return AE_OK;
        }
        cmd.write(0x80);
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b1, 0b1) {
            *Value = 0xFF;
            return AE_OK;
        }
        *Value = data.read() as i64;
    } else {
        //Write
        if !wait_for(0b10, 0) {
            return AE_OK;
        }
        cmd.write(0x81);
        if !wait_for(0b10, 0) {
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b10, 0) {
            return AE_OK;
        }
        data.write(*Value as u8);
    }

    acpica::AE_OK
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
pub unsafe extern "C" fn embedded_ctl_setup(
    _RegionHandle: acpica::ACPI_HANDLE,
    _Function: acpica::UINT32,
    _HandlerContext: *mut ::core::ffi::c_void,
    _RegionContext: *mut *mut ::core::ffi::c_void,
) -> acpica::ACPI_STATUS {
    acpica::AE_OK
}
