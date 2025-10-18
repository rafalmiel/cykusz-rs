#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsGetTimer() -> UINT64 {
    //100s ns
    crate::arch::dev::hpet::current_ns() as i64 / 100
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsSignal(Function: UINT32, Info: *mut ::core::ffi::c_void) -> ACPI_STATUS {
    if Function == ACPI_SIGNAL_FATAL as i32 {
        panic!("ACPI_SIGNAL_FATAL");
    }

    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsEnterSleep(
    SleepState: UINT8,
    RegaValue: UINT32,
    RegbValue: UINT32,
) -> ACPI_STATUS {
    AE_OK
}
