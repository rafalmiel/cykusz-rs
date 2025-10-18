#![allow(non_snake_case)]
#![allow(unused_variables)]

pub use acpica::*;

pub mod ec;
mod int;
mod mem;
mod memio;
mod misc;
mod pci;
mod portio;
mod print;
mod sync;
mod threads;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS {
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsTerminate() -> ACPI_STATUS {
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsGetRootPointer() -> ACPI_PHYSICAL_ADDRESS {
    let mut val = 0;
    // SAFE: Called from within ACPI init context
    match unsafe { AcpiFindRootPointer(&mut val) } {
        AE_OK => {}
        e @ _ => {
            println!("Failed to find ACPI root pointer : {}", e);

            return 0;
        }
    }

    val as ACPI_PHYSICAL_ADDRESS
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsPredefinedOverride(
    InitVal: *const ACPI_PREDEFINED_NAMES,
    NewVal: *mut ACPI_STRING,
) -> ACPI_STATUS {
    unsafe {
        *NewVal = 0 as *mut _;
    }
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsTableOverride(
    ExistingTable: *mut ACPI_TABLE_HEADER,
    NewTable: *mut *mut ACPI_TABLE_HEADER,
) -> ACPI_STATUS {
    unsafe {
        *NewTable = 0 as *mut _;
    }
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsPhysicalTableOverride(
    ExistingTable: *mut ACPI_TABLE_HEADER,
    NewAddress: *mut ACPI_PHYSICAL_ADDRESS,
    NewTableLength: *mut UINT32,
) -> ACPI_STATUS {
    unsafe {
        *NewAddress = 0;
    }

    AE_OK
}
