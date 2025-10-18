#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsReadPciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: *mut UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unsafe {
        //println!("PCI read: {:?} {} {}", *PciId, Reg, Width);
        *Value = crate::drivers::pci::read(
            (*PciId).Segment as u16,
            (*PciId).Bus as u16,
            (*PciId).Device as u16,
            (*PciId).Function as u16,
            Reg as u32,
            Width as u32,
        ) as i64;
    }
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsWritePciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unsafe {
        //println!("PCI write: {:?} {} {} {}", *PciId, Reg, Value, Width);
        crate::drivers::pci::write(
            (*PciId).Segment as u16,
            (*PciId).Bus as u16,
            (*PciId).Device as u16,
            (*PciId).Function as u16,
            Reg as u32,
            Value as u64,
            Width as u32,
        );
    }
    AE_OK
}
