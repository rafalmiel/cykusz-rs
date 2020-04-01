#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

use crate::arch::raw::cpuio::Port;

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsReadPort(
    Address: ACPI_IO_ADDRESS,
    Value: *mut UINT32,
    Width: UINT32,
) -> ACPI_STATUS {
    //println!("Read port 0x{:x} width: {}", Address, Width);
    unsafe {
        *Value = match Width {
            8 => Port::<u8>::new(Address as u16).read() as u32,
            16 => Port::<u16>::new(Address as u16).read() as u32,
            32 => Port::<u32>::new(Address as u16).read() as u32,
            _ => panic!("Unsupported port"),
        };

        AE_OK
    }
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsWritePort(
    Address: ACPI_IO_ADDRESS,
    Value: UINT32,
    Width: UINT32,
) -> ACPI_STATUS {
    //println!("Write port 0x{:x} value: {}, width: {}", Address, Value, Width);
    unsafe {
        match Width {
            8 => Port::<u8>::new(Address as u16).write(Value as u8),
            16 => Port::<u16>::new(Address as u16).write(Value as u16),
            32 => Port::<u32>::new(Address as u16).write(Value as u32),
            _ => panic!("Unsupported port"),
        }

        AE_OK
    }
}
