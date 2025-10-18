#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

use crate::kernel::mm::*;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsReadMemory(
    Address: ACPI_PHYSICAL_ADDRESS,
    Value: *mut UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unsafe {
        *Value = match Width {
            8 => PhysAddr(Address as usize).to_mapped().read::<u8>() as i64,
            16 => PhysAddr(Address as usize).to_mapped().read::<u16>() as i64,
            32 => PhysAddr(Address as usize).to_mapped().read::<u32>() as i64,
            64 => PhysAddr(Address as usize).to_mapped().read::<u64>() as i64,
            _ => panic!("Invalid Width"),
        };

        AE_OK
    }
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsWriteMemory(
    Address: ACPI_PHYSICAL_ADDRESS,
    Value: UINT64,
    Width: UINT32,
) -> ACPI_STATUS {
    unsafe {
        match Width {
            8 => PhysAddr(Address as usize)
                .to_mapped()
                .store::<u8>(Value as u8),
            16 => PhysAddr(Address as usize)
                .to_mapped()
                .store::<u16>(Value as u16),
            32 => PhysAddr(Address as usize)
                .to_mapped()
                .store::<u32>(Value as u32),
            64 => PhysAddr(Address as usize)
                .to_mapped()
                .store::<u64>(Value as u64),
            _ => panic!("Invalid Width"),
        };

        AE_OK
    }
}
