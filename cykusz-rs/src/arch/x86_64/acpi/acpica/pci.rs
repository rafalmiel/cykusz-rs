#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsReadPciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: *mut UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unsafe {
        *Value = crate::arch::dev::expci::read(
            (*PciId).Segment,
            (*PciId).Bus,
            (*PciId).Device,
            (*PciId).Function,
            Reg,
            Width,
        );
        //println!("PCI: R {:?} r 0x{:x} {}: {}", unsafe {*PciId}, Reg, Width, *Value);
    }
    AE_OK
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsWritePciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    //println!("PCI: W {:?} r 0x{:x} {}", unsafe {*PciId}, Reg, Width);
    unsafe {
        crate::arch::dev::expci::write(
            (*PciId).Segment,
            (*PciId).Bus,
            (*PciId).Device,
            (*PciId).Function,
            Reg,
            Value,
            Width,
        );
    }
    AE_OK
}
