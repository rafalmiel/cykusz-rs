#![allow(non_snake_case)]
#![allow(unused_variables)]

use alloc::boxed::Box;

use acpica::*;

use crate::kernel::sync::{Semaphore, Spin};

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsCreateLock(OutHandle: *mut *mut ::core::ffi::c_void) -> ACPI_STATUS {
    unsafe {
        let spin = Spin::<()>::new(());
        *OutHandle = Box::into_raw(Box::new(spin)) as *mut core::ffi::c_void;
    }
    AE_OK
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsDeleteLock(Handle: *mut ::core::ffi::c_void) {
    unsafe {
        let b = Box::from_raw(Handle as *mut Spin<()>);
        core::mem::drop(b)
    }
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsAcquireLock(Handle: *mut ::core::ffi::c_void) -> ACPI_SIZE {
    unsafe {
        let b = &*(Handle as *mut Spin<()>);
        b.unguarded_obtain();
        0
    }
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsReleaseLock(Handle: *mut ::core::ffi::c_void, Flags: ACPI_SIZE) {
    unsafe {
        let b = &*(Handle as *mut Spin<()>);
        b.unguarded_release();
    }
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsCreateSemaphore(
    MaxUnits: UINT32,
    InitialUnits: UINT32,
    OutHandle: *mut *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    let sem = Semaphore::new(InitialUnits as isize, MaxUnits as isize);
    unsafe {
        *OutHandle = Box::into_raw(Box::new(sem)) as *mut core::ffi::c_void;
    }

    AE_OK
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsDeleteSemaphore(Handle: *mut ::core::ffi::c_void) -> ACPI_STATUS {
    unsafe {
        let b = Box::from_raw(Handle as *mut Semaphore);
        core::mem::drop(b)
    }

    AE_OK
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsWaitSemaphore(
    Handle: *mut ::core::ffi::c_void,
    Units: UINT32,
    Timeout: UINT16,
) -> ACPI_STATUS {
    unsafe {
        let s = &*(Handle as *mut Semaphore);
        s.acquire().expect("[ ACPICA ] Signalled acpica thread");
    }

    AE_OK
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsSignalSemaphore(
    Handle: *mut ::core::ffi::c_void,
    Units: UINT32,
) -> ACPI_STATUS {
    unsafe {
        let s = &*(Handle as *mut Semaphore);
        s.release();
    }

    AE_OK
}
