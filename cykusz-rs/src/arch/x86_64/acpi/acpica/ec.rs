use acpica::*;

use crate::arch::x86_64::raw::cpuio::Port;
use crate::kernel::timer::busy_sleep;

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

    let mut global_lock_handle = 0;
    assert_eq!(
        acpica::AcpiAcquireGlobalLock(u16::max_value(), &mut global_lock_handle as *mut u32),
        acpica::AE_OK
    );

    if BitWidth != 8 {
        panic!("Unsupported BitWidth {}", BitWidth);
    }

    if Function == 0 {
        //Read
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        cmd.write(0x80);
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b1, 0b1) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        *Value = data.read() as u64;
    } else {
        //Write
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        cmd.write(0x81);
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(*Value as u8);
    }

    acpica::AcpiReleaseGlobalLock(global_lock_handle);
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
