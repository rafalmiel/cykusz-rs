use acpica::*;

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsReadPciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: *mut UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unimplemented!()
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsWritePciConfiguration(
    PciId: *mut ACPI_PCI_ID,
    Reg: UINT32,
    Value: UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unimplemented!()
}
